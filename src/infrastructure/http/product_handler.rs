use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tracing::error;

use crate::application::product_function_service::ProductFunctionService;
use crate::application::product_service::ProductService;
use crate::domain::device::ProductType;
use crate::domain::product::Product;
use crate::domain::product_function::ProductFunction;

/// Shared application state for product handlers.
#[derive(Clone)]
pub struct ProductState {
    pub product_service: Arc<ProductService>,
    pub function_service: Arc<ProductFunctionService>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    #[serde(rename = "productKey")]
    pub product_key: String,
    #[serde(default, rename = "productSecret")]
    pub product_secret: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, rename = "productVersion")]
    pub product_version: Option<String>,
    #[serde(default, rename = "productType")]
    pub product_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProductRequest {
    #[serde(default, rename = "productSecret")]
    pub product_secret: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, rename = "productVersion")]
    pub product_version: Option<String>,
    #[serde(default, rename = "productType")]
    pub product_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListProductsQuery {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, rename = "productType")]
    pub product_type: Option<String>,
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub size: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateFunctionRequest {
    pub identifier: String,
    pub method: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, rename = "callType")]
    pub call_type: Option<String>,
    #[serde(default, rename = "functionType")]
    pub function_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// GET /api/v1/products
pub async fn list_products(
    State(state): State<ProductState>,
    Query(query): Query<ListProductsQuery>,
) -> Result<Json<Value>, StatusCode> {
    let product_type = query
        .product_type
        .as_deref()
        .and_then(parse_product_type);
    let page = query.page.unwrap_or(1);
    let size = query.size.unwrap_or(20);

    match state
        .product_service
        .list_products(query.name.as_deref(), product_type.as_ref(), page, size)
        .await
    {
        Ok(products) => Ok(Json(serde_json::json!({
            "code": 200,
            "data": products,
            "page": page,
            "size": size,
            "total": products.len()
        }))),
        Err(e) => {
            error!(error = %e, "list_products failed");
            Ok(Json(serde_json::json!({
                "code": 500,
                "message": format!("internal error: {}", e)
            })))
        }
    }
}

/// GET /api/v1/products/{productKey}/functions
pub async fn list_product_functions(
    State(state): State<ProductState>,
    Path(product_key): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    match state.function_service.list_functions(&product_key).await {
        Ok(Some(functions)) => Ok(Json(serde_json::json!({
            "code": 200,
            "data": functions,
            "total": functions.len(),
        }))),
        Ok(None) => Ok(Json(serde_json::json!({
            "code": 404,
            "message": "product not found",
        }))),
        Err(e) => {
            error!(pk = %product_key, error = %e, "list_product_functions failed");
            Ok(Json(serde_json::json!({
                "code": 500,
                "message": format!("internal error: {}", e)
            })))
        }
    }
}

/// POST /api/v1/products/{productKey}/functions
pub async fn create_product_function(
    State(state): State<ProductState>,
    Path(product_key): Path<String>,
    Json(req): Json<CreateFunctionRequest>,
) -> Result<Json<Value>, StatusCode> {
    let func = ProductFunction {
        id: None,
        module_id: None,
        identifier: req.identifier,
        method: req.method,
        name: req.name,
        call_type: req.call_type,
        function_type: req.function_type,
        description: req.description,
        gmt_create: None,
        gmt_modified: None,
    };

    match state
        .function_service
        .create_function(&product_key, &func)
        .await
    {
        Ok(Some(function_id)) => Ok(Json(serde_json::json!({
            "code": 200,
            "message": "function created",
            "data": {
                "id": function_id,
            }
        }))),
        Ok(None) => Ok(Json(serde_json::json!({
            "code": 404,
            "message": "product not found",
        }))),
        Err(e) => {
            error!(pk = %product_key, error = %e, "create_product_function failed");
            Ok(Json(serde_json::json!({
                "code": 500,
                "message": format!("create failed: {}", e)
            })))
        }
    }
}

/// DELETE /api/v1/products/{productKey}/functions/{id}
pub async fn delete_product_function(
    State(state): State<ProductState>,
    Path((_product_key, function_id)): Path<(String, i64)>,
) -> Result<Json<Value>, StatusCode> {
    match state.function_service.delete_function(function_id).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "code": 200,
            "message": "function deleted",
        }))),
        Err(e) => {
            error!(func_id = %function_id, error = %e, "delete_product_function failed");
            Ok(Json(serde_json::json!({
                "code": 500,
                "message": format!("delete failed: {}", e)
            })))
        }
    }
}


/// GET /api/v1/products/{productKey}
pub async fn get_product(
    State(state): State<ProductState>,
    Path(product_key): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    match state.product_service.get_product(&product_key).await {
        Ok(Some(product)) => Ok(Json(serde_json::json!({
            "code": 200,
            "data": product,
        }))),
        Ok(None) => Ok(Json(serde_json::json!({
            "code": 404,
            "message": "product not found",
        }))),
        Err(e) => {
            error!(pk = %product_key, error = %e, "get_product failed");
            Ok(Json(serde_json::json!({
                "code": 500,
                "message": format!("internal error: {}", e)
            })))
        }
    }
}

/// POST /api/v1/products
pub async fn create_product(
    State(state): State<ProductState>,
    Json(req): Json<CreateProductRequest>,
) -> Result<Json<Value>, StatusCode> {
    let product_type = req.product_type.as_deref().and_then(parse_product_type);
    let product = Product {
        id: None,
        product_key: req.product_key.clone(),
        product_secret: req.product_secret,
        name: req.name,
        product_version: req.product_version,
        product_type,
        description: req.description,
        gmt_create: None,
        gmt_modified: None,
    };

    match state.product_service.create_product(&product).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "code": 200,
            "message": "product created",
            "data": {
                "productKey": product.product_key,
            }
        }))),
        Err(e) => {
            error!(pk = %product.product_key, error = %e, "create_product failed");
            Ok(Json(serde_json::json!({
                "code": 500,
                "message": format!("create failed: {}", e)
            })))
        }
    }
}

/// PUT /api/v1/products/{productKey}
pub async fn update_product(
    State(state): State<ProductState>,
    Path(product_key): Path<String>,
    Json(req): Json<UpdateProductRequest>,
) -> Result<Json<Value>, StatusCode> {
    let product_type = req.product_type.as_deref().and_then(parse_product_type);
    let product = Product {
        id: None,
        product_key: product_key.clone(),
        product_secret: req.product_secret,
        name: req.name,
        product_version: req.product_version,
        product_type,
        description: req.description,
        gmt_create: None,
        gmt_modified: None,
    };

    match state.product_service.update_product(&product_key, &product).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "code": 200,
            "message": "product updated",
        }))),
        Err(e) => {
            error!(pk = %product_key, error = %e, "update_product failed");
            Ok(Json(serde_json::json!({
                "code": 500,
                "message": format!("update failed: {}", e)
            })))
        }
    }
}

/// DELETE /api/v1/products/{productKey}
pub async fn delete_product(
    State(state): State<ProductState>,
    Path(product_key): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    match state.product_service.delete_product(&product_key).await {
        Ok(true) => Ok(Json(serde_json::json!({
            "code": 200,
            "message": "product deleted",
        }))),
        Ok(false) => Ok(Json(serde_json::json!({
            "code": 409,
            "message": "product has devices, cannot delete",
        }))),
        Err(e) => {
            error!(pk = %product_key, error = %e, "delete_product failed");
            Ok(Json(serde_json::json!({
                "code": 500,
                "message": format!("delete failed: {}", e)
            })))
        }
    }
}

fn parse_product_type(value: &str) -> Option<ProductType> {
    match value.to_lowercase().as_str() {
        "directdevice" => Some(ProductType::DirectDevice),
        "gateway" => Some(ProductType::Gateway),
        "subdevice" => Some(ProductType::SubDevice),
        "unrealdevice" => Some(ProductType::UnrealDevice),
        _ => None,
    }
}
