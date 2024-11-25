use reqwest::{ Client, Url };
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct ErrorReporter {
    client: Client,
    base_reporter_url: Url,
    service: Option<String>,
    subservice: Option<String>,
}

impl ErrorReporter {
    pub fn new_env() -> Option<Self> {
        let base_reporter_url = std::env::var("ERROR_REPORTER_URL")
            .ok()?
            .parse()
            .ok()?;
        Self::new(base_reporter_url)
    }

    pub fn new(base_reporter_url: Url) -> Option<Self> {
        let client = Client::builder().build().ok()?;
        Some(Self {
            client,
            base_reporter_url,
            service: None,
            subservice: None,
        })
    }

    pub fn set_service(&mut self, service: String) {
        self.service = Some(service);
    }
    pub fn set_subservice(&mut self, subservice: String) {
        self.subservice = Some(subservice);
    }

    fn with_slash(str: &str) -> String {
        str.split('/').chain(std::iter::once("/")).collect()
    }
    pub fn with_service(mut self, service: String) -> Self {
        self.service = Some(Self::with_slash(&service));
        self
    }
    pub fn with_subservice(mut self, subservice: String) -> Self {
        self.subservice = Some(subservice);
        self
    }

    pub fn url(&self) -> Option<Url> {
        let url = self.base_reporter_url.join("log/").ok()?;
        let url = url.join(self.service.as_deref().unwrap_or("default/")).ok()?;
        let url = url.join(self.subservice.as_deref().unwrap_or("default")).ok()?;
        Some(url)
    }
    pub async fn report(&self, message: &str, data: &impl Serialize) -> Result<(), String> {
        let Some(url) = self.url() else {
            return Err("Invalid internal URL".to_string());
        };
        let body = serde_json::to_string(data).map_err(|e| format!("Error serializing data: {:?}", e))?;
        let res = self.client.post(url)
            .query(&[("message", message)])
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await;
        match res {
            Ok(res) => {
                if !res.status().is_success() {
                    return match res.text().await {
                        Ok(text) => {
                            Err(format!("Error response: {:?}", text))
                        }
                        Err(e) => Err(format!("Error reading response: {:?}", e)),
                    };
                }
                match res.text().await {
                    Ok(text) => {
                        if text == "OK" {
                            Ok(())
                        } else {
                            Err(format!("Error response: {:?}", text))
                        }
                    }
                    Err(e) => Err(format!("Error reading response: {:?}", e)),
                }
            }
            Err(e) => Err(format!("Error sending request: {:?}", e)),
        }
    }
}

pub fn init(reporter: ErrorReporter) -> Result<(), String> {
    if let Err(e) = GLOBAL_REPORTER.set(reporter) {
        return Err(format!("Error setting global reporter: {:?}", e));
    }
    Ok(())
}

pub fn init_env(service: impl Into<String>, subservice: impl Into<String>) -> Result<(), String> {
    let reporter = ErrorReporter::new_env().ok_or("Error creating reporter from environment")?;
    let reporter = reporter
        .with_service(service.into())
        .with_subservice(subservice.into());
    init(reporter)
}

pub async fn report(message: &str, data: &impl Serialize) -> Result<(), String> {
    let reporter = GLOBAL_REPORTER.get().ok_or("Error getting global reporter")?;
    reporter.report(message, data).await
}

static GLOBAL_REPORTER: std::sync::OnceLock<ErrorReporter> = std::sync::OnceLock::new();
