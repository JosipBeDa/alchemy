use super::{
    contract::{RepositoryContract, ServiceContract},
    data::{GetUsersPaginated, UserResponse},
};
use crate::error::Error;
use actix_web::HttpResponse;
use alx_core::web::http::response::Response;
use async_trait::async_trait;
use reqwest::StatusCode;

pub(super) struct UserService<R>
where
    R: RepositoryContract,
{
    pub repo: R,
}

#[async_trait]
impl<R> ServiceContract for UserService<R>
where
    R: RepositoryContract,
{
    fn get_paginated(&self, data: GetUsersPaginated) -> Result<HttpResponse, Error> {
        let users = self.repo.get_paginated(
            data.page.unwrap_or(1_u16),
            data.per_page.unwrap_or(25),
            data.sort_by,
        )?;

        Ok(UserResponse::new(users)
            .to_response(StatusCode::OK)
            .finish())
    }
}
