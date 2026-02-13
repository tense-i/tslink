use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::application::function_param_service::FunctionParamService;
use crate::domain::function_param::FunctionParam;

#[derive(Clone)]
pub struct FunctionParamState {
    pub service: Arc<FunctionParamService>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(msg: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateFunctionParamRequest {
    pub product_id: i64,
    pub module_id: Option<i64>,
    pub function_id: i64,
    pub param_name: String,
    pub param_identifier: String,
    pub param_type: String,
    pub data_type: String,
    pub specs: Option<String>,
    pub rel_param_id: Option<i64>,
    #[serde(default)]
    pub required: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFunctionParamRequest {
    pub param_name: Option<String>,
    pub param_identifier: Option<String>,
    pub param_type: Option<String>,
    pub data_type: Option<String>,
    pub specs: Option<String>,
    pub rel_param_id: Option<i64>,
    pub required: Option<i32>,
}

pub async fn list_by_function(
    State(state): State<FunctionParamState>,
    Path(function_id): Path<i64>,
) -> impl IntoResponse {
    match state.service.list_by_function(function_id).await {
        Ok(params) => Json(ApiResponse::success(params)),
        Err(e) => Json(ApiResponse::<Vec<FunctionParam>>::error(e.to_string())),
    }
}

pub async fn list_by_product(
    State(state): State<FunctionParamState>,
    Path(product_id): Path<i64>,
) -> impl IntoResponse {
    match state.service.list_by_product(product_id).await {
        Ok(params) => Json(ApiResponse::success(params)),
        Err(e) => Json(ApiResponse::<Vec<FunctionParam>>::error(e.to_string())),
    }
}

pub async fn get_by_id(
    State(state): State<FunctionParamState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.service.get_by_id(id).await {
        Ok(Some(param)) => Json(ApiResponse::success(param)),
        Ok(None) => Json(ApiResponse::<FunctionParam>::error(format!(
            "Function param with id '{}' not found",
            id
        ))),
        Err(e) => Json(ApiResponse::<FunctionParam>::error(e.to_string())),
    }
}

pub async fn create(
    State(state): State<FunctionParamState>,
    Json(req): Json<CreateFunctionParamRequest>,
) -> impl IntoResponse {
    let param = FunctionParam {
        id: None,
        product_id: req.product_id,
        module_id: req.module_id,
        function_id: req.function_id,
        param_name: req.param_name,
        param_identifier: req.param_identifier,
        param_type: req.param_type,
        data_type: req.data_type,
        specs: req.specs,
        rel_param_id: req.rel_param_id,
        required: req.required,
        gmt_create: None,
        gmt_modified: None,
        gmt_create_by: None,
        gmt_modified_by: None,
    };

    match state.service.create(param).await {
        Ok(created) => Json(ApiResponse::success(created)),
        Err(e) => Json(ApiResponse::<FunctionParam>::error(e.to_string())),
    }
}

pub async fn update(
    State(state): State<FunctionParamState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateFunctionParamRequest>,
) -> impl IntoResponse {
    let existing = match state.service.get_by_id(id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return Json(ApiResponse::<bool>::error(format!(
                "Function param with id '{}' not found",
                id
            )))
        }
        Err(e) => return Json(ApiResponse::<bool>::error(e.to_string())),
    };

    let updated = FunctionParam {
        id: Some(id),
        product_id: existing.product_id,
        module_id: existing.module_id,
        function_id: existing.function_id,
        param_name: req.param_name.unwrap_or(existing.param_name),
        param_identifier: req.param_identifier.unwrap_or(existing.param_identifier),
        param_type: req.param_type.unwrap_or(existing.param_type),
        data_type: req.data_type.unwrap_or(existing.data_type),
        specs: req.specs.or(existing.specs),
        rel_param_id: req.rel_param_id.or(existing.rel_param_id),
        required: req.required.unwrap_or(existing.required),
        gmt_create: existing.gmt_create,
        gmt_modified: None,
        gmt_create_by: existing.gmt_create_by,
        gmt_modified_by: None,
    };

    match state.service.update(id, updated).await {
        Ok(success) => Json(ApiResponse::success(success)),
        Err(e) => Json(ApiResponse::<bool>::error(e.to_string())),
    }
}

pub async fn delete(
    State(state): State<FunctionParamState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.service.delete(id).await {
        Ok(success) => Json(ApiResponse::success(success)),
        Err(e) => Json(ApiResponse::<bool>::error(e.to_string())),
    }
}

pub async fn delete_by_function(
    State(state): State<FunctionParamState>,
    Path(function_id): Path<i64>,
) -> impl IntoResponse {
    match state.service.delete_by_function(function_id).await {
        Ok(count) => Json(ApiResponse::success(count)),
        Err(e) => Json(ApiResponse::<u64>::error(e.to_string())),
    }
}
