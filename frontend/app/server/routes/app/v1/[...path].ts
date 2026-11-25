// 开发期同源代理：浏览器只请求 :3000（同源，零 CORS、零系统代理干扰），
// /app/v1/** 由 Nitro 在服务端转发到 app BFF（node 直连不走系统代理，
// clash 这类系统级代理拦不到它）。生产部署是 nginx 同源，此路由不参与。
export default defineEventHandler(async (event) => {
    const target = 'http://127.0.0.1:10700'
    return proxyRequest(event, target + event.path)
})
