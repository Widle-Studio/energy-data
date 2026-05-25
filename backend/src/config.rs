use dotenvy::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub allowed_origins: Vec<String>,
    pub admin_token: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenv().ok(); // Ignore missing .env file

        let database_url = env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable must be set"))?;

        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .map_err(|_| anyhow::anyhow!("PORT must be a valid u16"))?;

        let allowed_origins = env::var("ALLOWED_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let admin_token = env::var("ADMIN_TOKEN").ok();

        if admin_token.is_none() {
            tracing::warn!(
                "ADMIN_TOKEN environment variable is not set. Admin endpoints will be inaccessible."
            );
        }

        Ok(Self {
            database_url,
            port,
            allowed_origins,
            admin_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    lazy_static::lazy_static! {
        static ref ENV_MUTEX: Mutex<()> = Mutex::new(());
    }

    #[test]
    fn test_from_env_missing_database_url() -> anyhow::Result<()> {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _ = std::fs::rename(".env", ".env.bak");
        unsafe {
            env::remove_var("DATABASE_URL");
            env::set_var("PORT", "8080");
        }
        let result = Config::from_env();
        let _ = std::fs::rename(".env.bak", ".env");
        match result {
            Err(e) => assert_eq!(
                e.to_string(),
                "DATABASE_URL environment variable must be set"
            ),
            Ok(_) => panic!("Expected error for missing DATABASE_URL, but got success"),
        }
        Ok(())
    }

    #[test]
    fn test_from_env_success() -> anyhow::Result<()> {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _ = std::fs::rename(".env", ".env.bak");
        unsafe {
            env::set_var("DATABASE_URL", "postgres://user:pass@localhost:5432/db");
            env::set_var("PORT", "9000");
            env::set_var("ADMIN_TOKEN", "secret-token");
        }
        let result = Config::from_env();
        let _ = std::fs::rename(".env.bak", ".env");
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(
            config.database_url,
            "postgres://user:pass@localhost:5432/db"
        );
        assert_eq!(config.port, 9000);
        assert_eq!(config.admin_token, Some("secret-token".to_string()));
        Ok(())
    }

    #[test]
    fn test_from_env_default_port() -> anyhow::Result<()> {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            env::set_var("DATABASE_URL", "postgres://user:pass@localhost:5432/db");
            env::remove_var("PORT");
        }
        let config = Config::from_env()?;
        assert_eq!(config.port, 8080);
        Ok(())
    }

    #[test]
    fn test_from_env_invalid_port() -> anyhow::Result<()> {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            env::set_var("DATABASE_URL", "postgres://user:pass@localhost:5432/db");
            env::set_var("PORT", "invalid");
        }
        let result = Config::from_env();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("PORT must be a valid u16")
        );
        Ok(())
    }

    #[test]
    fn test_from_env_allowed_origins() -> anyhow::Result<()> {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            env::set_var("DATABASE_URL", "postgres://user:pass@localhost:5432/db");
            env::set_var("PORT", "8080"); // Ensure PORT is valid so it doesn't fail
            env::set_var(
                "ALLOWED_ORIGINS",
                "http://localhost:3000, https://energtx.app ",
            );
        }
        let config = Config::from_env()?;
        assert_eq!(
            config.allowed_origins,
            vec![
                "http://localhost:3000".to_string(),
                "https://energtx.app".to_string()
            ]
        );

        unsafe {
            env::set_var("ALLOWED_ORIGINS", "");
        }
        let config = Config::from_env()?;
        assert!(config.allowed_origins.is_empty());

        unsafe {
            env::remove_var("ALLOWED_ORIGINS");
        }
        let config = Config::from_env()?;
        assert!(config.allowed_origins.is_empty());
        Ok(())
    }
}
