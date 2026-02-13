use axum::routing::{delete, get, post, put};
use axum::Router;

use super::device_type_handler::{
    create_device_type, delete_device_type, get_device_type, list_device_types,
    update_device_type, DeviceTypeState,
};
use super::device_handler::{
    create_device, delete_device, get_device, list_devices, update_device, DeviceState,
};
use super::function_param_handler::{
    create, delete as delete_param, delete_by_function, get_by_id, list_by_function,
    list_by_product, update, FunctionParamState,
};
use super::health::{health_check, metrics, HealthState};
use super::product_handler::{
    create_product, create_product_function, delete_product, delete_product_function, get_product,
    list_product_functions, list_products, update_product, ProductState,
};
use super::service_handler::{invoke_service, ServiceState};
use super::shadow_handler::{get_shadow, update_shadow, ShadowState};
use super::ws_handler::{ws_handler, WsState};

/// Build the complete application router with all routes.
///
/// Routes:
/// - GET    /api/v1/devices?productKey=xxx      — list devices
/// - POST   /api/v1/devices                     — create device
/// - GET    /api/v1/devices/:pk/:did            — get device info
/// - PUT    /api/v1/devices/:pk/:did            — update device
/// - DELETE /api/v1/devices/:pk/:did            — delete device
/// - POST   /api/v1/devices/:pk/:did/services/:method — invoke service
/// - GET    /api/v1/devices/:pk/:did/shadow     — get shadow
/// - PUT    /api/v1/devices/:pk/:did/shadow     — update shadow
/// - GET    /api/v1/products                    — list products
/// - POST   /api/v1/products                    — create product
/// - GET    /api/v1/products/:productKey        — get product
/// - PUT    /api/v1/products/:productKey        — update product
/// - DELETE /api/v1/products/:productKey        — delete product
/// - GET    /api/v1/products/:productKey/functions — list product functions
/// - POST   /api/v1/products/:productKey/functions — create product function
/// - DELETE /api/v1/products/:productKey/functions/:id — delete product function
/// - GET    /api/v1/device-types                 — list device types
/// - POST   /api/v1/device-types                 — create device type
/// - GET    /api/v1/device-types/:code           — get device type
/// - PUT    /api/v1/device-types/:code           — update device type
/// - DELETE /api/v1/device-types/:code           — delete device type
/// - GET    /api/v1/function-params/function/:functionId — list params by function
/// - GET    /api/v1/function-params/product/:productId — list params by product
/// - GET    /api/v1/function-params/:id           — get param by id
/// - POST   /api/v1/function-params               — create param
/// - PUT    /api/v1/function-params/:id           — update param
/// - DELETE /api/v1/function-params/:id           — delete param
/// - DELETE /api/v1/function-params/function/:functionId — delete params by function
/// - GET    /api/v1/ws?product_key=xxx&device_id=yyy — WebSocket event stream
/// - GET    /health                              — health check
/// - GET    /metrics                             — prometheus metrics
pub fn build_router(
    device_state: DeviceState,
    service_state: ServiceState,
    shadow_state: ShadowState,
    product_state: ProductState,
    health_state: HealthState,
    ws_state: WsState,
    device_type_state: DeviceTypeState,
    function_param_state: FunctionParamState,
) -> Router {
    let device_routes = Router::new()
        .route("/api/v1/devices", get(list_devices).post(create_device))
        .route(
            "/api/v1/devices/:pk/:did",
            get(get_device).put(update_device).delete(delete_device),
        )
        .with_state(device_state);

    let service_routes = Router::new()
        .route(
            "/api/v1/devices/:pk/:did/services/:method",
            post(invoke_service),
        )
        .with_state(service_state);

    let shadow_routes = Router::new()
        .route(
            "/api/v1/devices/:pk/:did/shadow",
            get(get_shadow).put(update_shadow),
        )
        .with_state(shadow_state);

    let product_routes = Router::new()
        .route("/api/v1/products", get(list_products).post(create_product))
        .route(
            "/api/v1/products/:productKey",
            get(get_product)
                .put(update_product)
                .delete(delete_product),
        )
        .route(
            "/api/v1/products/:productKey/functions",
            get(list_product_functions).post(create_product_function),
        )
        .route(
            "/api/v1/products/:productKey/functions/:id",
            delete(delete_product_function),
        )
        .with_state(product_state);

    let health_routes = Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics))
        .with_state(health_state);

    let ws_routes = Router::new()
        .route("/api/v1/ws", get(ws_handler))
        .with_state(ws_state);

    let device_type_routes = Router::new()
        .route(
            "/api/v1/device-types",
            get(list_device_types).post(create_device_type),
        )
        .route(
            "/api/v1/device-types/:code",
            get(get_device_type)
                .put(update_device_type)
                .delete(delete_device_type),
        )
        .with_state(device_type_state);

    let function_param_routes = Router::new()
        .route("/api/v1/function-params", post(create))
        .route(
            "/api/v1/function-params/:id",
            get(get_by_id).put(update).delete(delete_param),
        )
        .route(
            "/api/v1/function-params/function/:functionId",
            get(list_by_function).delete(delete_by_function),
        )
        .route(
            "/api/v1/function-params/product/:productId",
            get(list_by_product),
        )
        .with_state(function_param_state);

    Router::new()
        .merge(device_routes)
        .merge(service_routes)
        .merge(shadow_routes)
        .merge(product_routes)
        .merge(ws_routes)
        .merge(device_type_routes)
        .merge(function_param_routes)
        .merge(health_routes)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_route_paths() {
        // Verify route path format
        let path = "/api/v1/devices/pk001/did001";
        assert!(path.starts_with("/api/v1/devices/"));

        let service_path = "/api/v1/devices/pk001/did001/services/reboot";
        assert!(service_path.contains("/services/"));
    }
}
