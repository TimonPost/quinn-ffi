initSidebarItems({"enum":[["ServerName","Encodes ways a client can know the expected name of the server."]],"struct":[["CertificateTransparencyPolicy","Policy for enforcing Certificate Transparency."],["ClientConfig","Common configuration for (typically) all connections made by a program."],["ClientConnection","This represents a single TLS client connection."],["ClientConnectionData","State associated with a client connection."],["ClientSessionMemoryCache","An implementer of `StoresClientSessions` that stores everything in memory.  It enforces a limit on the number of entries to bound memory usage."],["DangerousClientConfig","Accessor for dangerous configuration options."],["HandshakeSignatureValid","Marker types.  These are used to bind the fact some verification (certificate chain or handshake signature) has taken place into protocol states.  We use this to have the compiler check that there are no ‘goto fail’-style elisions of important checks before we reach the traffic stage."],["InvalidDnsNameError","The provided input could not be parsed because it is not a syntactically-valid DNS Name."],["NoClientSessionStorage","An implementer of `StoresClientSessions` which does nothing."],["ServerCertVerified","Zero-sized marker type representing verification of a server cert chain."],["WantsClientCert","A config builder state where the caller needs to supply whether and how to provide a client certificate."],["WantsTransparencyPolicyOrClientCert","A config builder state where the caller needs to supply a certificate transparency policy or client certificate resolver."],["WebPkiVerifier","Default `ServerCertVerifier`, see the trait impl for more information."],["WriteEarlyData","Stub that implements io::Write and dispatches to `write_early_data`."]],"trait":[["ClientQuicExt","Methods specific to QUIC client sessions"],["ResolvesClientCert","A trait for the ability to choose a certificate chain and private key for the purposes of client authentication."],["ServerCertVerifier","Something that can verify a server certificate chain, and verify signatures made by certificates."],["StoresClientSessions","A trait for the ability to store client session data. The keys and values are opaque."]]});