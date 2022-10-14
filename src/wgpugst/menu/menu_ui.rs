use iced_native::{
    command::Command,
    program::Program,
    widget::{button, column::Column, row::Row, slider, text::Text, Container},
    Alignment, Color, Element, Length,
};
use iced_wgpu::Renderer;

pub enum Gameview {
    Loading,
    Loaded,
}
#[derive(Debug, Clone)]
pub enum Message {
    Loaded,
    StartHost,
}

impl Gameview {
    pub fn new() -> (Gameview, Command<Message>) {
        (
            Gameview::Loading,
            Command::perform(SavedState::load(), Message::Loaded),
        )
    }
    pub fn title(&self) -> String {
        let dirty = match self {
            Gameview::Loading => false,
            Gameview::Loaded => true,
        };

        format!("Gameview {}- WGPU", if dirty { "Loading... " } else { "" })
    }
}
impl Program for Gameview {
    type Renderer = Renderer;
    type Message = Message;

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Loading => {
                self.background_color = color;
            }
            Message::TextChanged(text) => {
                self.text = text;
            }
            Message::ButtonPressed => {}
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Renderer> {
        todo!()
    }
}
