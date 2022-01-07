use crate::{ffi::{
    Out,
    QuinnResult,
}, RustlsServerConfigHandle, RustlsClientConfigHandle};
use rustls::{
    Certificate,
    KeyLogFile,
    PrivateKey,
    RootCertStore,
    client::{ServerCertVerified, ServerCertVerifier}
};
use std::{
    fs,
    sync::Arc,
};

use crate::proto::{ServerConfig, ClientConfig};

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

#[no_mangle]
pub extern "cdecl" fn default_server_config(
    mut out_handle: Out<RustlsServerConfigHandle>,
) -> QuinnResult {
    // tracing::subscriber::set_global_default(
    //     tracing_subscriber::FmtSubscriber::builder()
    //         .with_env_filter("trace")
    //         .finish(),
    // )
    // .unwrap();

    let (key, cert) = generate_self_signed_cert("cert.der", "key.der");

    let (key, cert) = (PrivateKey(key), Certificate(cert));
    let mut store = RootCertStore::empty();
    store.add(&cert);

    let mut config = rustls::ServerConfig::builder()
        .with_safe_default_cipher_suites()
        .with_safe_default_kx_groups()
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .unwrap();

    config.key_log = Arc::new(KeyLogFile::new());

    let config = ServerConfig::with_crypto(Arc::new(config));

    unsafe { out_handle.init(RustlsServerConfigHandle::alloc(ServerConfig::from(config))) }

    QuinnResult::ok()
}

#[no_mangle]
pub extern "cdecl" fn default_client_config(
    mut out_handle: Out<RustlsClientConfigHandle>,
) -> QuinnResult {
    let mut crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(SkipServerVerification::new())
        .with_no_client_auth();

    crypto.key_log = Arc::new(KeyLogFile::new());

    unsafe { out_handle.init(RustlsClientConfigHandle::alloc(ClientConfig::new(Arc::new(crypto)))); }

    QuinnResult::ok()
}

struct SkipServerVerification;

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item=&[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
}
