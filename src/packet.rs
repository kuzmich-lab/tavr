pub const MAGIC: [u8; 2] = [0xA5, 0x5A];
pub const HEADER_LEN: usize = 5;
pub const CRC_LEN: usize = 2;
pub const MAX_PAYLOAD_LEN: usize = 256 - HEADER_LEN - CRC_LEN;

pub mod msg_type {
    pub const NAV_DATA: u8 = 0x01;
    pub const MESSAGE: u8 = 0x02;
    pub const ACK: u8 = 0x03;
    pub const JOIN: u8 = 0x04;
    pub const PING: u8 = 0x05;
    pub const PONG: u8 = 0x06;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    TooShort,
    BadMagic,
    PayloadTooLong,
    CrcMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    pub device_id: [u8; 2],
    pub msg_type: u8,
    pub payload: [u8; MAX_PAYLOAD_LEN],
    pub payload_len: usize,
}

impl Packet {
    pub fn new(device_id: [u8; 2], msg_type: u8) -> Self {
        Self {
            device_id,
            msg_type,
            payload: [0; MAX_PAYLOAD_LEN],
            payload_len: 0,
        }
    }

    pub fn with_payload(mut self, payload: &[u8]) -> Result<Self, DecodeError> {
        if payload.len() > MAX_PAYLOAD_LEN {
            return Err(DecodeError::PayloadTooLong);
        }
        self.payload_len = payload.len();
        self.payload[..payload.len()].copy_from_slice(payload);
        Ok(self)
    }

    pub fn total_len(&self) -> usize {
        HEADER_LEN + self.payload_len + CRC_LEN
    }

    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, DecodeError> {
        let total = self.total_len();
        if buf.len() < total {
            return Err(DecodeError::TooShort);
        }

        let p = &mut buf[..total];

        p[0..2].copy_from_slice(&MAGIC);
        p[3..4].copy_from_slice(&self.device_id[0].to_le_bytes());
        p[4..5].copy_from_slice(&self.device_id[1].to_le_bytes());
        p[5] = self.msg_type;
        p[10] = self.payload_len as u8;

        p[HEADER_LEN..HEADER_LEN + self.payload_len]
            .copy_from_slice(&self.payload[..self.payload_len]);

        let crc = crc16_xmodem(&p[..HEADER_LEN + self.payload_len]);
        p[HEADER_LEN + self.payload_len..total].copy_from_slice(&crc.to_le_bytes());

        Ok(total)
    }

    pub fn encode_to_array(&self) -> Result<([u8; 256], usize), DecodeError> {
        let mut buf = [0u8; 256];
        let len = self.encode(&mut buf)?;
        Ok((buf, len))
    }

    pub fn decode(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() < HEADER_LEN + CRC_LEN {
            return Err(DecodeError::TooShort);
        }

        if data[0..2] != MAGIC {
            return Err(DecodeError::BadMagic);
        }

        let device_id = [data[3], data[4]];
        let msg_type = data[5];
        let payload_len = data[10] as usize;

        if payload_len > MAX_PAYLOAD_LEN {
            return Err(DecodeError::PayloadTooLong);
        }

        let total = HEADER_LEN + payload_len + CRC_LEN;
        if data.len() < total {
            return Err(DecodeError::TooShort);
        }

        let received_crc =
            u16::from_le_bytes(data[HEADER_LEN + payload_len..total].try_into().unwrap());
        let computed_crc = crc16_xmodem(&data[..HEADER_LEN + payload_len]);
        if received_crc != computed_crc {
            return Err(DecodeError::CrcMismatch);
        }

        let mut payload = [0u8; MAX_PAYLOAD_LEN];
        payload[..payload_len].copy_from_slice(&data[HEADER_LEN..HEADER_LEN + payload_len]);

        Ok(Self {
            device_id,
            msg_type,
            payload,
            payload_len,
        })
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload[..self.payload_len]
    }
}

const CRC16_TABLE: [u16; 256] = {
    let mut table = [0u16; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = (i as u16) << 8;
        let mut bit = 0;
        while bit < 8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
            bit += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
};

pub fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        crc = (crc << 8) ^ CRC16_TABLE[((crc >> 8) as u8 ^ byte) as usize];
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let payload = [0x01, 0x02, 0x03, 0xFF, 0x42];
        let pkt = Packet::new([0x12, 0x34], msg_type::MESSAGE)
            .with_payload(&payload)
            .unwrap();

        let (buf, len) = pkt.encode_to_array().unwrap();
        assert!(len == HEADER_LEN + payload.len() + CRC_LEN);

        let decoded = Packet::decode(&buf[..len]).unwrap();
        assert_eq!(pkt, decoded);
    }

    #[test]
    fn test_crc_mismatch() {
        let payload = [0x01, 0x02, 0x03];
        let pkt = Packet::new([0x00, 0x01], msg_type::PING)
            .with_payload(&payload)
            .unwrap();

        let (mut buf, len) = pkt.encode_to_array().unwrap();
        // Портим один байт payload
        buf[HEADER_LEN] ^= 0xFF;

        let result = Packet::decode(&buf[..len]);
        assert_eq!(result, Err(DecodeError::CrcMismatch));
    }

    #[test]
    fn test_bad_magic() {
        let data = [0x00u8; 20];
        assert_eq!(Packet::decode(&data), Err(DecodeError::BadMagic));
    }

    #[test]
    fn test_fragmentation() {
        let data = vec![0xAB; 300]; // больше одного пакета
        let (packets, count) = fragment_data(0x0001, msg_type::SENSOR_DATA, 7, &data).unwrap();
        assert_eq!(count, 2);
        assert_eq!(packets[0].fragment_index, 0);
        assert_eq!(packets[0].total_fragments, 2);
        assert!(packets[0].has_flag(flags::FRAGMENTED));
        assert!(!packets[0].is_last_fragment());
        assert_eq!(packets[1].fragment_index, 1);
        assert_eq!(packets[1].total_fragments, 2);
        assert!(packets[1].is_last_fragment());

        for i in 0..count {
            let (buf, len) = packets[i].encode_to_array().unwrap();
            let decoded = Packet::decode(&buf[..len]).unwrap();
            assert_eq!(packets[i], decoded);
        }
    }

    #[test]
    fn test_empty_payload() {
        let pkt = Packet::new([0x00, 0x42], msg_type::PING);
        let (buf, len) = pkt.encode_to_array().unwrap();
        assert_eq!(len, HEADER_LEN + CRC_LEN);

        let decoded = Packet::decode(&buf[..len]).unwrap();
        assert_eq!(pkt, decoded);
        assert_eq!(decoded.payload_len, 0);
    }

    #[test]
    fn test_max_payload() {
        let payload = [0xFF; MAX_PAYLOAD_LEN];
        let pkt = Packet::new([0xFF, 0xFF], msg_type::SENSOR_DATA)
            .with_payload(&payload)
            .unwrap();
        let (buf, len) = pkt.encode_to_array().unwrap();
        assert_eq!(len, 256);

        let decoded = Packet::decode(&buf[..len]).unwrap();
        assert_eq!(pkt, decoded);
    }

    #[test]
    fn test_payload_too_long() {
        let big = [0x00; MAX_PAYLOAD_LEN + 1];
        let result = Packet::new([0, 1], 1).with_payload(&big);
        assert_eq!(result, Err(DecodeError::PayloadTooLong));
    }
}
