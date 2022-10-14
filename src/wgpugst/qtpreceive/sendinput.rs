use bytes::Bytes;
use flexbuffers::Builder;
use qp2p::Connection;
use winit::dpi::PhysicalPosition;

pub async fn send_mouse<'a>(
    mouse_state: u32,
    cursor_position: &'a PhysicalPosition<f64>,
    sent_stream_input: &'a Connection,
    mouse_data: u32,
) {
    let mut builder = Builder::default();
    let mut send_input = builder.start_map();
    send_input.push("mouse_state", mouse_state as u32);
    send_input.push("position_x", cursor_position.x as i16);
    send_input.push("position_y", cursor_position.y as i16);
    send_input.push("mouse_data", mouse_data as u8);
    send_input.end_map();
    // println!(
    //     "Mouse state :{} x :{} y :{}",
    //     *mouse_state_temp, cursor_position_temp.x, cursor_position_temp.y
    // );
    sent_stream_input
        .send(Bytes::copy_from_slice(builder.view()))
        .await
        .expect("get error sent input");
}
pub async fn send_keyboard<'a>(
    keyboard_state: u16,
    keyscan: u16,
    sent_stream_input: &'a Connection,
) {
    let mut builder = Builder::default();
    let mut send_input = builder.start_map();
    send_input.push("keyscan", keyscan);
    send_input.push("keyboard_state", keyboard_state);
    send_input.end_map();
    // println!(
    //     "Mouse state :{} x :{} y :{}",
    //     *mouse_state_temp, cursor_position_temp.x, cursor_position_temp.y
    // );
    sent_stream_input
        .send(Bytes::copy_from_slice(builder.view()))
        .await
        .expect("get error sent input");
}
