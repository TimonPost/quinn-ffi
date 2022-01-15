use crate::ffi::{
    FFIResult,
    Out,
};
use rustls::{
    client::{
        ServerCertVerified,
        ServerCertVerifier,
    },
    Certificate,
    KeyLogFile,
    PrivateKey,
    RootCertStore,
};
use std::{
    fs,
    sync::Arc,
};

use crate::{
    ffi::Handle,
    proto::{
        ClientConfig,
        ServerConfig,
    },
};
use std::sync::Mutex;

pub fn generate_self_signed_cert(cert_path: &str, key_path: &str) -> (Vec<u8>, Vec<u8>) {
    // Generate dummy certificate.
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let serialized_key = certificate.serialize_private_key_der();
    let serialized_certificate = certificate.serialize_der().unwrap();

    // Write to files.
    fs::write(&cert_path, &serialized_certificate).expect("failed to write certificate");
    fs::write(&key_path, &serialized_key).expect("failed to write private key");

    (serialized_key, serialized_certificate)
}

pub struct SkipServerVerification;

impl SkipServerVerification {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
}
