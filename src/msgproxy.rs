use crate::{MESSAGE_PBC, MsgType, RECEIVE_CHANNEL, SEND_CHANNEL};
use crate::{Packet, packet};
use defmt::info;

#[embassy_executor::task]
pub async fn msgproxy() {
    let pub0 = MESSAGE_PBC.publisher().unwrap();
    let mut sub0 = MESSAGE_PBC.subscriber().unwrap();
    let sender = SEND_CHANNEL.sender();
    let receiver = RECEIVE_CHANNEL.receiver();
    let mut send_buf = [0u8; 118];
    loop {
        let msg = sub0.try_next_message_pure();
        if msg.is_some() {
            let pack: Packet = Packet::new(msg.unwrap());
            let total = pack.encode(&mut send_buf).unwrap();
            info!("send_buf: {}", send_buf[..total]);
            sender.send(send_buf).await;
        }

        match receiver.try_receive() {
            Ok(_) => {
                let mut msg = packet::decode(&mut send_buf).unwrap();
                match msg.msg_type {
                    MsgType::Ping => {
                        msg.msg_type = MsgType::Pong;
                        pub0.publish_immediate(msg);
                    }
                    MsgType::Pong => {
                        msg.msg_type = MsgType::Message;
                        pub0.publish_immediate(msg);
                    }
                    _ => {}
                }
            }
            Err(_) => {}
        }
    }
}
