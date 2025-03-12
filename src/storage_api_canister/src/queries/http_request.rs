use ic_http_certification::{HttpRequest, HttpResponse};

pub type Args = HttpRequest<'static>;
pub type Response = HttpResponse<'static>;
