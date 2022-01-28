initSidebarItems({"constant":[["DEFAULT_SUPPORTED_VERSIONS","The QUIC protocol version implemented."]],"enum":[["ConfigError","Errors in the configuration of an endpoint"],["ConnectError","Errors in the parameters being used to create a new connection"],["ConnectionError","Reasons why a connection might be lost"],["DatagramEvent","Event resulting from processing a single datagram"],["Dir","Whether a stream communicates data in both directions or only from the initiator"],["EcnCodepoint","Explicit congestion notification codepoint"],["Event","Events of interest to the application"],["FinishError","Reasons why attempting to finish a stream might fail"],["ReadError","Errors triggered when reading from a recv stream"],["ReadableError","Errors triggered when opening a recv stream for reading"],["SendDatagramError","Errors that can arise when sending a datagram"],["Side","Whether an endpoint was the initiator of a connection"],["StreamEvent","Application events about streams"],["WriteError","Errors triggered while writing to a send stream"]],"mod":[["congestion","Logic for controlling the rate at which data is sent"],["crypto","Traits and implementations for the QUIC cryptography protocol"],["generic","Types that are generic over the crypto protocol implementation"],["transport_parameters","QUIC connection transport parameters"]],"struct":[["ApplicationClose","Reason given by an application for closing the connection"],["Chunk","A chunk of data from the receive stream"],["Chunks","Chunks"],["ClientConfig","Configuration for outgoing connections"],["Connection","Protocol state and logic for a single QUIC connection"],["ConnectionClose","Reason given by the transport for closing the connection"],["ConnectionEvent","Events sent from an Endpoint to a Connection"],["ConnectionHandle","Internal identifier for a `Connection` currently associated with an endpoint"],["ConnectionId","Protocol-level identifier for a connection."],["ConnectionStats","Connection statistics"],["Datagram","An unreliable datagram"],["Datagrams","API to control datagram traffic"],["Endpoint","The main entry point to the library"],["EndpointConfig","Global configuration for the endpoint, affecting all connections"],["EndpointEvent","Events sent from a Connection to an Endpoint"],["IdleTimeout","Maximum duration of inactivity to accept before timing out the connection."],["RandomConnectionIdGenerator","Generates purely random connection IDs of a certain length"],["RecvStream","Access to streams"],["SendStream","Access to streams"],["ServerConfig","Parameters governing incoming connections"],["StreamId","Identifier for a stream within a particular connection"],["Streams","Access to streams"],["Transmit","An outgoing packet"],["TransportConfig","Parameters governing the core QUIC state machine"],["TransportError","Transport-level errors occur when a peer violates the protocol specification"],["TransportErrorCode","Transport-level error code"],["UnknownStream","Error indicating that a stream has not been opened or has already been finished or reset"],["VarInt","An integer less than 2^62"],["VarIntBoundsExceeded","Error returned when constructing a `VarInt` from a value >= 2^62"],["Written","Indicates how many bytes and chunks had been transferred in a write operation"]],"trait":[["BytesSource","A source of one or more buffers which can be converted into `Bytes` buffers on demand"],["ConnectionIdGenerator","Generates connection IDs for incoming connections"]]});