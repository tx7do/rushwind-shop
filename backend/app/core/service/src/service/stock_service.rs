//! The stock services — the low-stock alert ledger and the operation
//! log (the reference's stock_alert_service.go / stock_op_log_service.go).
//! Both are write-ledger faces: the alerts land when stock crosses the
//! threshold (wired from the sku AdjustStock flow), the op logs append
//! per stock movement.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::{stock_alerts, stock_op_logs};
use store::paging::fetch_paged;

use shop_proto::proto::catalog::service::v1 as catalogv1;
use shop_proto::proto::pagination::PagingRequest;

fn alert_proto(r: stock_alerts::Model) -> catalogv1::StockAlert {
    catalogv1::StockAlert {
        id: Some(r.id),
        sku_id: Some(r.sku_id),
        stock_qty_at_trigger: Some(r.stock_qty_at_trigger),
        threshold: Some(r.threshold),
        status: Some(
            if r.status == "RESOLVED" {
                catalogv1::stock_alert::Status::Resolved
            } else {
                catalogv1::stock_alert::Status::Open
            } as i32,
        ),
        created_at: r.created_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

fn op_log_proto(r: stock_op_logs::Model) -> catalogv1::StockOpLog {
    catalogv1::StockOpLog {
        id: Some(r.id),
        sku_id: Some(r.sku_id),
        product_id: r.product_id,
        delta: Some(r.delta),
        stock_before: r.stock_before,
        stock_after: r.stock_after,
        reason: Some(
            catalogv1::stock_op_log::Reason::try_from(match r.reason.as_deref() {
                Some("CANCEL_RESTORE") => 2,
                Some("REFUND_RESTORE") => 3,
                Some("MANUAL_ADJUST") => 4,
                _ => 1,
            })
            .unwrap_or(catalogv1::stock_op_log::Reason::OrderDeduct) as i32,
        ),
        ref_id: r.ref_id,
        remark: r.remark.clone(),
        created_at: r.created_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

pub struct StockAlertService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl catalogv1::stock_alert_service_server::StockAlertService for StockAlertService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::ListStockAlertResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            stock_alerts::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::ListStockAlertResponse {
            items: rows.into_iter().map(alert_proto).collect(),
            total: total as i64,
        }))
    }


    async fn get(
        &self,
        request: Request<catalogv1::GetStockAlertRequest>,
    ) -> Result<Response<catalogv1::StockAlert>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::get_stock_alert_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = stock_alerts::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("stock alert not found"))?;
        Ok(Response::new(alert_proto(row)))
    }

    async fn update(
        &self,
        request: Request<catalogv1::UpdateStockAlertRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let req = request.into_inner();
        let data = req.data.ok_or_else(|| bad("data required"))?;
        let existing = stock_alerts::Entity::find_by_id(req.id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("stock alert not found"))?;
        let mut active: stock_alerts::ActiveModel = existing.into();
        if let Some(v) = data.status {
            active.status = Set(
                if v == catalogv1::stock_alert::Status::Resolved as i32 {
                    "RESOLVED"
                } else {
                    "OPEN"
                }
                .to_string(),
            );
        }
        active.updated_at = Set(Some(store::now().into()));
        active.update(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

}

/// The internal append face (the sku AdjustStock flow calls this when
/// the ledger wiring lands; the service trait carries List/Count/Get
/// only).
pub async fn append_op_log(
    db: &sea_orm::DatabaseConnection,
    sku_id: i64,
    product_id: Option<i64>,
    delta: i32,
    stock_before: i32,
    stock_after: i32,
    reason: &str,
    remark: Option<String>,
) -> Result<(), Status> {
    let active = stock_op_logs::ActiveModel {
        sku_id: Set(sku_id),
        product_id: Set(product_id),
        delta: Set(delta),
        stock_before: Set(Some(stock_before)),
        stock_after: Set(Some(stock_after)),
        reason: Set(Some(reason.to_string())),
        remark: Set(remark),
        created_at: Set(Some(store::now().into())),
        ..Default::default()
    };
    active.insert(db).await.map_err(db_status)?;
    Ok(())
}

pub struct StockOpLogService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl catalogv1::stock_op_log_service_server::StockOpLogService for StockOpLogService {
    async fn list(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::ListStockOpLogResponse>, Status> {
        let (rows, total) = fetch_paged(
            &self.state.db,
            stock_op_logs::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::ListStockOpLogResponse {
            items: rows.into_iter().map(op_log_proto).collect(),
            total: total as i64,
        }))
    }

    async fn count(
        &self,
        request: Request<PagingRequest>,
    ) -> Result<Response<catalogv1::CountStockOpLogResponse>, Status> {
        let (_, total) = fetch_paged(
            &self.state.db,
            stock_op_logs::Entity::find(),
            &request.into_inner(),
        )
        .await
        .map_err(|e| Status::internal(e.message))?;
        Ok(Response::new(catalogv1::CountStockOpLogResponse { count: total as i64 }))
    }

    async fn get(
        &self,
        request: Request<catalogv1::GetStockOpLogRequest>,
    ) -> Result<Response<catalogv1::StockOpLog>, Status> {
        let req = request.into_inner();
        let Some(catalogv1::get_stock_op_log_request::QueryBy::Id(id)) = req.query_by else {
            return Err(bad("query_by required"));
        };
        let row = stock_op_logs::Entity::find_by_id(id)
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("stock op log not found"))?;
        Ok(Response::new(op_log_proto(row)))
    }

}
