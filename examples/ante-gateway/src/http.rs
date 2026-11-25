use tracing::debug;

fn webpki_root_certificates() -> Vec<reqwest::Certificate> {
    webpki_root_certs::TLS_SERVER_ROOT_CERTS
        .iter()
        .filter_map(|cert| match reqwest::Certificate::from_der(cert.as_ref()) {
            Ok(cert) => Some(cert),
            Err(err) => {
                debug!(%err, "Failed to parse root certificate; skipping");
                None
            }
        })
        .collect()
}

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .tls_certs_merge(webpki_root_certificates())
        .build()
        .expect("failed to build reqwest client with bundled roots")
}
