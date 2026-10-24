//! L7 payload fixtures: real-format DNS, HTTP, TLS, QUIC, DHCP and mDNS messages.

/// DNS query for `example.com` (A record), transaction id 0x1234.
pub fn dns_query_example_com() -> Vec<u8> {
    let mut message = Vec::new();
    message.extend_from_slice(&[0x12, 0x34]); // id
    message.extend_from_slice(&[0x01, 0x00]); // flags: standard query, recursion desired
    message.extend_from_slice(&[0x00, 0x01]); // questions
    message.extend_from_slice(&[0x00, 0x00]); // answers
    message.extend_from_slice(&[0x00, 0x00]); // authority
    message.extend_from_slice(&[0x00, 0x00]); // additional
    message.extend_from_slice(&encode_name("example.com"));
    message.extend_from_slice(&[0x00, 0x01]); // QTYPE A
    message.extend_from_slice(&[0x00, 0x01]); // QCLASS IN
    message
}

/// DNS response for `example.com` with one A record (93.184.216.34), using a
/// compression pointer for the answer name — the common real-world shape.
pub fn dns_response_example_com() -> Vec<u8> {
    let mut message = Vec::new();
    message.extend_from_slice(&[0x12, 0x34]);
    message.extend_from_slice(&[0x81, 0x80]); // response, recursion available
    message.extend_from_slice(&[0x00, 0x01]); // questions
    message.extend_from_slice(&[0x00, 0x01]); // answers
    message.extend_from_slice(&[0x00, 0x00]);
    message.extend_from_slice(&[0x00, 0x00]);
    message.extend_from_slice(&encode_name("example.com"));
    message.extend_from_slice(&[0x00, 0x01]);
    message.extend_from_slice(&[0x00, 0x01]);
    // Answer: pointer to offset 12 (the question name)
    message.extend_from_slice(&[0xc0, 0x0c]);
    message.extend_from_slice(&[0x00, 0x01]); // TYPE A
    message.extend_from_slice(&[0x00, 0x01]); // CLASS IN
    message.extend_from_slice(&[0x00, 0x00, 0x01, 0x2c]); // TTL 300
    message.extend_from_slice(&[0x00, 0x04]); // RDLENGTH
    message.extend_from_slice(&[93, 184, 216, 34]);
    message
}

/// mDNS query for `_services._dns-sd._udp.local`.
pub fn mdns_query() -> Vec<u8> {
    let mut message = Vec::new();
    message.extend_from_slice(&[0x00, 0x00]); // mDNS uses id 0
    message.extend_from_slice(&[0x00, 0x00]); // standard query
    message.extend_from_slice(&[0x00, 0x01]);
    message.extend_from_slice(&[0x00, 0x00]);
    message.extend_from_slice(&[0x00, 0x00]);
    message.extend_from_slice(&[0x00, 0x00]);
    message.extend_from_slice(&encode_name("_services._dns-sd._udp.local"));
    message.extend_from_slice(&[0x00, 0x0c]); // QTYPE PTR
    message.extend_from_slice(&[0x00, 0x01]);
    message
}

/// Encodes a domain name into DNS label format.
fn encode_name(name: &str) -> Vec<u8> {
    let mut encoded = Vec::new();
    for label in name.split('.') {
        encoded.push(label.len() as u8);
        encoded.extend_from_slice(label.as_bytes());
    }
    encoded.push(0);
    encoded
}

pub fn http_get_request() -> Vec<u8> {
    b"GET /index.html HTTP/1.1\r\nHost: example.com\r\nUser-Agent: cognton-test\r\nAccept: */*\r\n\r\n"
        .to_vec()
}

pub fn http_response_200() -> Vec<u8> {
    b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 5\r\n\r\nhello".to_vec()
}

/// TLS 1.2+ ClientHello record advertising SNI `example.com`.
pub fn tls_client_hello_example_com() -> Vec<u8> {
    let host = b"example.com";

    // server_name extension body: list_len(2) type(1) name_len(2) host
    let mut sni_body = Vec::new();
    sni_body.extend_from_slice(&((host.len() + 3) as u16).to_be_bytes());
    sni_body.push(0x00); // host_name
    sni_body.extend_from_slice(&(host.len() as u16).to_be_bytes());
    sni_body.extend_from_slice(host);

    let mut extensions = Vec::new();
    extensions.extend_from_slice(&[0x00, 0x00]); // extension type: server_name
    extensions.extend_from_slice(&(sni_body.len() as u16).to_be_bytes());
    extensions.extend_from_slice(&sni_body);

    let mut body = Vec::new();
    body.extend_from_slice(&[0x03, 0x03]); // client_version TLS 1.2
    body.extend_from_slice(&[0x11; 32]); // random
    body.push(0x00); // session id length
    body.extend_from_slice(&[0x00, 0x02]); // cipher suites length
    body.extend_from_slice(&[0x13, 0x01]); // TLS_AES_128_GCM_SHA256
    body.push(0x01); // compression methods length
    body.push(0x00); // null compression
    body.extend_from_slice(&(extensions.len() as u16).to_be_bytes());
    body.extend_from_slice(&extensions);

    let mut handshake = Vec::new();
    handshake.push(0x01); // ClientHello
    let length = body.len();
    handshake.extend_from_slice(&[
        ((length >> 16) & 0xff) as u8,
        ((length >> 8) & 0xff) as u8,
        (length & 0xff) as u8,
    ]);
    handshake.extend_from_slice(&body);

    let mut record = Vec::new();
    record.push(0x16); // handshake
    record.extend_from_slice(&[0x03, 0x01]); // record version TLS 1.0 (as browsers send)
    record.extend_from_slice(&(handshake.len() as u16).to_be_bytes());
    record.extend_from_slice(&handshake);
    record
}

/// TLS application data record (no SNI available).
pub fn tls_application_data() -> Vec<u8> {
    let mut record = vec![0x17, 0x03, 0x03];
    record.extend_from_slice(&16_u16.to_be_bytes());
    record.extend_from_slice(&[0x42; 16]);
    record
}

/// QUIC v1 Initial packet header (long header, DCID 8 bytes, SCID 4 bytes).
pub fn quic_initial() -> Vec<u8> {
    let mut packet = Vec::new();
    packet.push(0xc0); // long header, fixed bit, Initial
    packet.extend_from_slice(&0x0000_0001_u32.to_be_bytes()); // QUIC v1
    packet.push(0x08); // DCID length
    packet.extend_from_slice(&[0xab; 8]);
    packet.push(0x04); // SCID length
    packet.extend_from_slice(&[0xcd; 4]);
    packet.extend_from_slice(&[0x00; 32]); // token length + encrypted remainder
    packet
}

/// QUIC 1-RTT short-header packet.
pub fn quic_short_header() -> Vec<u8> {
    let mut packet = vec![0x40]; // short header with fixed bit set
    packet.extend_from_slice(&[0x5a; 32]);
    packet
}

/// DHCP Discover (BOOTP request + option 53 = 1).
pub fn dhcp_discover() -> Vec<u8> {
    let mut message = vec![0_u8; 236];
    message[0] = 0x01; // op: BOOTREQUEST
    message[1] = 0x01; // htype: Ethernet
    message[2] = 0x06; // hlen
    message[4..8].copy_from_slice(&[0xde, 0xad, 0xbe, 0xef]); // xid
    message.extend_from_slice(&[0x63, 0x82, 0x53, 0x63]); // magic cookie
    message.extend_from_slice(&[53, 1, 1]); // option 53: DHCP Discover
    message.extend_from_slice(&[55, 2, 1, 3]); // option 55: parameter request list
    message.push(255); // end
    message
}

/// DHCP ACK (BOOTP reply + option 53 = 5).
pub fn dhcp_ack() -> Vec<u8> {
    let mut message = vec![0_u8; 236];
    message[0] = 0x02; // op: BOOTREPLY
    message[1] = 0x01;
    message[2] = 0x06;
    message.extend_from_slice(&[0x63, 0x82, 0x53, 0x63]);
    message.extend_from_slice(&[53, 1, 5]); // DHCP ACK
    message.push(255);
    message
}
