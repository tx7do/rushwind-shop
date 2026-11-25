//! The invoice service — the per-order invoice cards (the reference's
//! invoice_service.go): the VAT/electronic type, the buyer tuple, the
//! amount, and the PENDING/ISSUED/CANCELLED lifecycle. The stored
//! spellings follow the demo data (the bare member names).

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::invoices;
use store::paging::fetch_paged;

use shop_proto::proto::invoice::service::v1 as invoicev1;
use shop_proto::proto::pagination::PagingRequest;

fn invoice_type_to_str(v: i32) -> &'static str {
    use invoicev1::invoice::InvoiceType;
    match invoicev1::invoice::InvoiceType::try_from(v).ok() {
        Some(InvoiceType::VatSpecial) => "VAT_SPECIAL",
        Some(InvoiceType::Electronic) => "ELECTRONIC",
        _ => "VAT_GENERAL",
    }
}

fn status_to_i32(stored: &str) -> i32 {
    use invoicev1::invoice::Status;
    (match stored {
        "ISSUED" => Status::Issued,
        "CANCELLED" => Status::Cancelled,
        _ => Status::Pending,
    }) as i32
}

fn status_to_str(v: i32) -> Option<&'static str> {
    use invoicev1::invoice::Status;
    Some(match Status::try_from(v).ok()? {
        Status::Pending => "PENDING",
        Status::Issued => "ISSUED",
        Status::Cancelled => "CANCELLED",
        _ => return None,
    })
}

fn invoice_proto(r: invoices::Model) -> invoicev1::Invoice {
    invoicev1::Invoice {
        id: Some(r.id),
        order_id: Some(r.order_id),
        user_id: r.user_id,
        invoice_number: r.invoice_number.clone(),
        invoice_type: Some(
            invoicev1::invoice::InvoiceType::try_from(match r.invoice_type.as_str() {
                "VAT_SPECIAL" => 2,
                "ELECTRONIC" => 3,
                _ => 1,
            })
            .unwrap_or(invoicev1::invoice::InvoiceType::VatGeneral) as i32,
        ),
        status: Some(status_to_i32(&r.status)),
        buyer_name: r.buyer_name.clone(),
        buyer_tax_id: r.buyer_tax_id.clone(),
        buyer_address: r.buyer_address.clone(),
        buyer_phone: r.buyer_phone.clone(),
        amount: Some(r.amount),
        currency: Some(r.currency.clone()),
        tenant_id: r.tenant_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

pub struct InvoiceService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl invoicev1::invoice_service_server::InvoiceService for InvoiceService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<invoicev1::ListInvoiceResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            invoices::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(invoicev1::ListInvoiceResponse {
            items: rows.into_iter().map(invoice_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<invoicev1::CountInvoiceResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            invoices::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(invoicev1::CountInvoiceResponse {
            count: total as i64,
        }))
    }

    async fn get(
        &self,
        request: Request<invoicev1::GetInvoiceRequest>,
    ) -> Result<Response<invoicev1::Invoice>, Status> {
        let req = request.into_inner();
        let Some(invoicev1::get_invoice_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = invoices::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("invoice not found"))?;
        Ok(Response::new(invoice_proto(row)))
    }

    async fn create(
        &self,
        request: Request<invoicev1::CreateInvoiceRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let order_id = data.order_id.ok_or_else(|| bad("data.orderId required"))?;
        // The invoice number: INV + the order id + a short random
        // suffix (the real tax bureau number rides the integration).
        let number = format!(
            "INV{:06}-{}",
            order_id,
            &uuid::Uuid::new_v4().simple().to_string()[..6]
        );
        let active = invoices::ActiveModel {
            order_id: Set(order_id),
            user_id: Set(data.user_id),
            invoice_number: Set(Some(number)),
            invoice_type: Set(invoice_type_to_str(data.invoice_type.unwrap_or(1)).to_string()),
            status: Set(data
                .status
                .and_then(status_to_str)
                .unwrap_or("PENDING")
                .to_string()),
            buyer_name: Set(data.buyer_name.clone()),
            buyer_tax_id: Set(data.buyer_tax_id.clone()),
            buyer_address: Set(data.buyer_address.clone()),
            buyer_phone: Set(data.buyer_phone.clone()),
            amount: Set(data.amount.unwrap_or(0)),
            currency: Set(data.currency.unwrap_or_else(|| "CNY".to_string())),
            tenant_id: Set(Some(data.tenant_id.unwrap_or(0))),
            created_at: Set(Some(store::now().into())),
            ..Default::default()
        };
        active.insert(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn update(
        &self,
        request: Request<invoicev1::UpdateInvoiceRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = invoices::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("invoice not found"))?;
        let mut active: invoices::ActiveModel = existing.into();
        if let Some(v) = data.invoice_number.clone() {
            active.invoice_number = Set(Some(v));
        }
        if let Some(v) = data.status.and_then(status_to_str) {
            active.status = Set(v.to_string());
        }
        if let Some(v) = data.buyer_name.clone() {
            active.buyer_name = Set(Some(v));
        }
        if let Some(v) = data.buyer_tax_id.clone() {
            active.buyer_tax_id = Set(Some(v));
        }
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn delete(
        &self,
        request: Request<invoicev1::DeleteInvoiceRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let Some(invoicev1::delete_invoice_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        invoices::Entity::delete_by_id(id)
            .exec(&self.state.db)
            .await
            .map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }
}
