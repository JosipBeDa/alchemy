use crate::cache::{contracts::SimpleCacheAccess, AuthID as CacheKey};
use crate::config::constants::{
    EMAIL_THROTTLE_DURATION, OTP_THROTTLE_DURATION, OTP_TOKEN_DURATION,
    REGISTRATION_TOKEN_DURATION, RESET_PW_TOKEN_DURATION, SESSION_CACHE_DURATION,
    WRONG_PASSWORD_CACHE_DURATION,
};
use crate::db::models::session;
use crate::error::Error;
use chrono::Utc;
use hextacy::drivers::Connect;
use hextacy::{adapt, contract};

adapt! {
    AuthenticationCache,
    use Driver for Connection as driver;
    Cache: SimpleCacheAccess<Connection>
}

#[contract]
impl<D, C, Cache> AuthenticationCache<D, C>
where
    C: Send,
    D: Connect<Connection = C> + Send + Sync,
    Cache: SimpleCacheAccess<C> + Send + Sync,
{
    /// Sessions get cached behind the user's csrf token.
    async fn set_session(&self, session_id: &str, session: &session::Session) -> Result<(), Error> {
        let mut conn = self.driver.connect().await?;
        Cache::set_json(
            &mut conn,
            CacheKey::Session,
            session_id,
            session,
            Some(SESSION_CACHE_DURATION),
        )
        .await
        .map_err(Error::new)
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), Error> {
        let mut conn = self.driver.connect().await?;
        Cache::delete(&mut conn, CacheKey::Session, session_id)
            .await
            .map_err(Error::new)
    }

    // Get

    async fn get_registration_token(&self, token: &str) -> Result<String, Error> {
        let mut conn = self.driver.connect().await?;
        Cache::get_string(&mut conn, CacheKey::RegToken, token)
            .await
            .map_err(Error::new)
    }

    async fn get_pw_token(&self, token: &str) -> Result<String, Error> {
        let mut conn = self.driver.connect().await?;
        Cache::get_string(&mut conn, CacheKey::PWToken, token)
            .await
            .map_err(Error::new)
    }

    async fn get_otp_token(&self, token: &str) -> Result<String, Error> {
        let mut conn = self.driver.connect().await?;
        Cache::get_string(&mut conn, CacheKey::OTPToken, token)
            .await
            .map_err(Error::new)
    }

    async fn get_otp_throttle(&self, token: &str) -> Result<i64, Error> {
        let mut conn = self.driver.connect().await?;
        Cache::get_i64(&mut conn, CacheKey::OTPThrottle, token)
            .await
            .map_err(Error::new)
    }

    async fn get_otp_attempts(&self, token: &str) -> Result<i64, Error> {
        let mut conn = self.driver.connect().await?;
        Cache::get_i64(&mut conn, CacheKey::OTPAttempts, token)
            .await
            .map_err(Error::new)
    }

    // Delete

    async fn delete_registration_token(&self, token: &str) -> Result<(), Error> {
        let mut conn = self.driver.connect().await?;
        Cache::delete(&mut conn, CacheKey::RegToken, token)
            .await
            .map_err(Error::new)
    }

    async fn delete_pw_token(&self, token: &str) -> Result<(), Error> {
        let mut conn = self.driver.connect().await?;
        Cache::delete(&mut conn, CacheKey::PWToken, token)
            .await
            .map_err(Error::new)
    }

    async fn delete_otp_token(&self, token: &str) -> Result<(), Error> {
        let mut conn = self.driver.connect().await?;
        Cache::delete(&mut conn, CacheKey::OTPToken, token)
            .await
            .map_err(Error::new)
    }

    async fn delete_otp_attempts(&self, token: &str) -> Result<(), Error> {
        let mut conn = self.driver.connect().await?;
        Cache::delete(&mut conn, CacheKey::OTPAttempts, token)
            .await
            .map_err(Error::new)
    }

    // Set

    async fn set_registration_token(&self, token: &str, value: &str) -> Result<(), Error> {
        let mut conn = self.driver.connect().await?;
        Cache::set_str(
            &mut conn,
            CacheKey::RegToken,
            token,
            value,
            Some(REGISTRATION_TOKEN_DURATION),
        )
        .await
        .map_err(Error::new)
    }

    async fn set_pw_token(&self, token: &str, value: &str) -> Result<(), Error> {
        let mut conn = self.driver.connect().await?;
        Cache::set_str(
            &mut conn,
            CacheKey::PWToken,
            token,
            value,
            Some(RESET_PW_TOKEN_DURATION),
        )
        .await
        .map_err(Error::new)
    }

    async fn set_otp_token(&self, token: &str, value: &str) -> Result<(), Error> {
        let mut conn = self.driver.connect().await?;
        Cache::set_str(
            &mut conn,
            CacheKey::OTPToken,
            token,
            value,
            Some(OTP_TOKEN_DURATION),
        )
        .await
        .map_err(Error::new)
    }

    /// Caches the number of login attempts using the user ID as the key. If the attempts do not exist they
    /// will be created, otherwise they will be incremented.
    async fn cache_login_attempt(&self, user_id: &str) -> Result<i64, Error> {
        let mut conn = self.driver.connect().await?;
        Cache::set_or_increment(
            &mut conn,
            CacheKey::LoginAttempts,
            user_id,
            1,
            Some(WRONG_PASSWORD_CACHE_DURATION),
        )
        .await
        .map_err(Error::new)
    }

    /// Removes the user's login attempts from the cache
    async fn delete_login_attempts(&self, user_id: &str) -> Result<(), Error> {
        let mut conn = self.driver.connect().await?;
        Cache::delete(&mut conn, CacheKey::LoginAttempts, user_id)
            .await
            .map_err(Error::new)
    }

    /// Cache the OTP throttle and attempts. The throttle always gets set to now and the attempts always get
    /// incremented. The domain should take care of the actual throttling.
    async fn cache_otp_throttle(&self, user_id: &str) -> Result<(), Error> {
        let mut conn = self.driver.connect().await?;

        let attempts = Cache::get_i64(&mut conn, CacheKey::OTPAttempts, user_id).await;

        match attempts {
            Ok(attempts) => {
                Cache::set_i64(
                    &mut conn,
                    CacheKey::OTPThrottle,
                    user_id,
                    Utc::now().timestamp(),
                    Some(OTP_THROTTLE_DURATION),
                )
                .await?;

                Cache::set_i64(
                    &mut conn,
                    CacheKey::OTPAttempts,
                    user_id,
                    attempts + 1,
                    Some(OTP_THROTTLE_DURATION),
                )
                .await?;
            }
            Err(_) => {
                Cache::set_i64(
                    &mut conn,
                    CacheKey::OTPThrottle,
                    user_id,
                    Utc::now().timestamp(),
                    Some(OTP_THROTTLE_DURATION),
                )
                .await?;

                Cache::set_i64(
                    &mut conn,
                    CacheKey::OTPAttempts,
                    user_id,
                    1,
                    Some(OTP_THROTTLE_DURATION),
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn set_email_throttle(&self, user_id: &str) -> Result<(), Error> {
        let mut conn = self.driver.connect().await?;
        Cache::set_i64(
            &mut conn,
            CacheKey::EmailThrottle,
            user_id,
            1,
            Some(EMAIL_THROTTLE_DURATION),
        )
        .await
        .map_err(Error::new)
    }

    async fn get_email_throttle(&self, user_id: &str) -> Result<i64, Error> {
        let mut conn = self.driver.connect().await?;
        Cache::get_i64(&mut conn, CacheKey::EmailThrottle, user_id)
            .await
            .map_err(Error::new)
    }
}