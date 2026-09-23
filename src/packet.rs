use chrono::NaiveTime;

use crate::MAX_PAYLOAD_LEN;
use crate::Message;
use crate::MsgType;
use crate::packet::msg_type::ACK;
use crate::packet::msg_type::MESSAGE;
use crate::packet::msg_type::NAV_DATA;
use crate::packet::msg_type::NONE;
use crate::packet::msg_type::PING;
use crate::packet::msg_type::PONG;

pub const MAGIC: [u8; 2] = [0xA5, 0x5A];
pub const HEADER_LEN: usize = 16;
pub const CRC_LEN: usize = 2;

//* Структура пакета
//|magic|src|dst|msg_type|payload_len|payload|CRC|
//|  2  | 6 | 6 |    1   |     1     | 0..200| 2 |
//*

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    TooShort,
    BadMagic,
    PayloadTooLong,
    CrcMismatch,
}

pub mod msg_type {
    pub const NONE: u8 = 0x00;
    pub const NAV_DATA: u8 = 0x01;
    pub const MESSAGE: u8 = 0x02;
    pub const ACK: u8 = 0x03;
    pub const PING: u8 = 0x04;
    pub const PONG: u8 = 0x05;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    pub src: [u8; 6],
    pub dst: [u8; 6],
    pub msg_type: u8,
    pub payload_len: usize,
    pub payload: [u8; MAX_PAYLOAD_LEN],
}

impl Packet {
    pub fn new(message: Message) -> Self {
        let msg_type = match message.msg_type {
            MsgType::None => NONE,
            MsgType::NavData => NAV_DATA,
            MsgType::Message => MESSAGE,
            MsgType::Ack => ACK,
            MsgType::Ping => PING,
            MsgType::Pong => PONG,
        };

        Self {
            src: message.source,
            dst: message.destination,
            msg_type,
            payload_len: message.message.len(),
            payload: message.message,
        }
    }

    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, DecodeError> {
        let total = HEADER_LEN + self.payload_len + CRC_LEN;
        if buf.len() < total {
            return Err(DecodeError::TooShort);
        }
        let p = &mut buf[..total];
        p[0..2].copy_from_slice(&MAGIC);
        p[2..8].copy_from_slice(&self.src.as_slice());
        p[8..14].copy_from_slice(&self.dst.as_slice());
        p[14] = self.msg_type;
        p[15] = self.payload_len as u8;
        p[HEADER_LEN..HEADER_LEN + self.payload_len]
            .copy_from_slice(&self.payload[..self.payload_len]);
        let crc = crc16_xmodem(&p[..HEADER_LEN + self.payload_len]);
        p[HEADER_LEN + self.payload_len..total].copy_from_slice(&crc.to_le_bytes());
        Ok(total)
    }

    // pub fn encode_to_array(&self) -> Result<([u8; 256], usize), DecodeError> {
    //     let mut buf = [0u8; 256];
    //     let len = self.encode(&mut buf)?;
    //     Ok((buf, len))
    // }

    // pub fn payload(&self) -> &[u8] {
    //     &self.payload[..self.payload_len]
    // }
}

pub fn decode(data: &[u8]) -> Result<Message, DecodeError> {
    if data.len() < HEADER_LEN + CRC_LEN {
        return Err(DecodeError::TooShort);
    }
    if data[0..2] != MAGIC {
        return Err(DecodeError::BadMagic);
    }
    let src = [data[2], data[3], data[4], data[5], data[6], data[7]];
    let dst = [data[8], data[9], data[10], data[11], data[12], data[13]];
    let msg_type = data[14];
    let payload_len = data[15] as usize;
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

    let msg_t: MsgType = match msg_type {
        NONE => MsgType::None,
        NAV_DATA => MsgType::NavData,
        MESSAGE => MsgType::Message,
        ACK => MsgType::Ack,
        PING => MsgType::Ping,
        PONG => MsgType::Pong,
        _ => MsgType::None,
    };

    Ok(Message {
        source: src,
        destination: dst,
        msg_type: msg_t,
        message: payload,
        time_stamp: NaiveTime::MIN,
    })
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
