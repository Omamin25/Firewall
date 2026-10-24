use crate::model::L7Detail;

const HEADER_LEN: usize = 12;
const FLAG_RESPONSE: u16 = 0x8000;
const MAX_LABELS: usize = 128;
const MAX_RECORDS: usize = 16;

const TYPE_A: u16 = 1;
const TYPE_AAAA: u16 = 28;
const TYPE_CNAME: u16 = 5;
const TYPE_PTR: u16 = 12;

/// Parses a DNS message (RFC 1035) into question and answer names.
/// Returns `None` when the header is impossible, so the caller can fall through
/// to another classifier rather than mislabelling the packet.
pub fn parse(payload: &[u8]) -> Option<L7Detail> {
    let message = Message::parse(payload)?;
    Some(L7Detail::Dns {
        is_response: message.is_response,
        questions: message.questions,
        answers: message.answers,
    })
}

/// mDNS uses the same wire format as DNS on port 5353.
pub fn parse_mdns(payload: &[u8]) -> Option<L7Detail> {
    let message = Message::parse(payload)?;
    Some(L7Detail::Mdns {
        is_response: message.is_response,
        questions: if message.questions.is_empty() {
            message.answers
        } else {
            message.questions
        },
    })
}

struct Message {
    is_response: bool,
    questions: Vec<String>,
    answers: Vec<String>,
}

impl Message {
    fn parse(payload: &[u8]) -> Option<Self> {
        let header = payload.get(..HEADER_LEN)?;
        let flags = be_u16(header, 2)?;
        let question_count = be_u16(header, 4)?;
        let answer_count = be_u16(header, 6)?;

        // Opcode 0..=5 only; anything else is not DNS and likely a false port match.
        if (flags >> 11) & 0x0F > 5 {
            return None;
        }

        let mut cursor = HEADER_LEN;
        let mut questions = Vec::new();
        for _ in 0..question_count.min(MAX_RECORDS as u16) {
            let (name, next) = read_name(payload, cursor)?;
            // QTYPE + QCLASS
            cursor = next.checked_add(4)?;
            if !name.is_empty() {
                questions.push(name);
            }
        }

        let mut answers = Vec::new();
        for _ in 0..answer_count.min(MAX_RECORDS as u16) {
            let Some((name, next)) = read_name(payload, cursor) else {
                break;
            };
            let record_type = match be_u16(payload, next) {
                Some(value) => value,
                None => break,
            };
            let Some(data_len) = be_u16(payload, next + 8).map(usize::from) else {
                break;
            };
            let data_start = next + 10;
            let data_end = match data_start.checked_add(data_len) {
                Some(end) if end <= payload.len() => end,
                _ => break,
            };
            if let Some(rendered) = render_record(payload, record_type, data_start, data_end, &name)
            {
                answers.push(rendered);
            }
            cursor = data_end;
        }

        Some(Self {
            is_response: flags & FLAG_RESPONSE != 0,
            questions,
            answers,
        })
    }
}

fn render_record(
    payload: &[u8],
    record_type: u16,
    start: usize,
    end: usize,
    name: &str,
) -> Option<String> {
    let data = payload.get(start..end)?;
    match record_type {
        TYPE_A => match data {
            [a, b, c, d] => Some(format!("{name} A {a}.{b}.{c}.{d}")),
            _ => None,
        },
        TYPE_AAAA if data.len() == 16 => {
            let mut segments = [0_u16; 8];
            for (index, segment) in segments.iter_mut().enumerate() {
                *segment = be_u16(data, index * 2)?;
            }
            Some(format!(
                "{name} AAAA {}",
                std::net::Ipv6Addr::from(segments)
            ))
        }
        TYPE_CNAME => read_name(payload, start).map(|(target, _)| format!("{name} CNAME {target}")),
        TYPE_PTR => read_name(payload, start).map(|(target, _)| format!("{name} PTR {target}")),
        _ => None,
    }
}

/// Reads a possibly-compressed domain name. Returns the decoded name plus the offset
/// immediately after the name *in the original record* (pointers do not advance it).
fn read_name(payload: &[u8], start: usize) -> Option<(String, usize)> {
    let mut name = String::new();
    let mut cursor = start;
    let mut after_pointer = None;
    let mut budget = MAX_LABELS;

    loop {
        budget = budget.checked_sub(1)?;
        let length = *payload.get(cursor)?;
        match length & 0xC0 {
            0x00 => {
                cursor += 1;
                if length == 0 {
                    let end = after_pointer.unwrap_or(cursor);
                    return Some((name, end));
                }
                let label_end = cursor.checked_add(usize::from(length))?;
                let label = payload.get(cursor..label_end)?;
                if !name.is_empty() {
                    name.push('.');
                }
                name.push_str(&String::from_utf8_lossy(label));
                cursor = label_end;
            }
            0xC0 => {
                let pointer = usize::from(be_u16(payload, cursor)? & 0x3FFF);
                // A pointer must point backwards; forward/self pointers are malformed.
                if pointer >= cursor {
                    return None;
                }
                after_pointer.get_or_insert(cursor + 2);
                cursor = pointer;
            }
            // 0x40/0x80 label types are reserved and not seen in practice.
            _ => return None,
        }
    }
}

fn be_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let pair = bytes.get(offset..offset + 2)?;
    Some(u16::from_be_bytes([pair[0], pair[1]]))
}
