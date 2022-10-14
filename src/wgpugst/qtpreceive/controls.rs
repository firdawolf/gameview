use iced_native::{
    command::Command,
    program::Program,
    widget::{button, column::Column, row::Row, slider, text::Text, Container},
    Alignment, Color, Element, Length,
};
use iced_wgpu::Renderer;
// use iced_winit::widget::TextInput;
// use iced_winit::widget::{ Column, Row, Text};

pub struct Controls {
    background_color: Color,
    text: String,
    buttonstate: button::State,
}

#[derive(Debug, Clone)]
pub enum Message {
    BackgroundColorChanged(Color),
    TextChanged(String),
    ButtonPressed,
}

impl Controls {
    pub fn new() -> Controls {
        let state = button::State::new();
        Controls {
            background_color: Color::BLACK,
            text: Default::default(),
            buttonstate: state,
        }
    }

    pub fn background_color(&self) -> Color {
        self.background_color
    }
    pub fn setText(&mut self, text1: i128) {
        self.text = text1.to_string();
    }
}

impl Program for Controls {
    type Renderer = Renderer;
    type Message = Message;

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::BackgroundColorChanged(color) => {
                self.background_color = color;
            }
            Message::TextChanged(text) => {
                self.text = text;
            }
            Message::ButtonPressed => {}
        }

        Command::none()
    }

    fn view(self: &Controls) -> Element<Message, Renderer> {
        let background_color = self.background_color;
        let text = &self.text;

        //let surface_windows = Compositor::new(settings, compatible_window)
        Container::new(
            Row::new()
                .width(Length::Fill)
                .height(Length::Fill)
                .align_items(Alignment::Start)
                .push(
                    Column::new()
                        .width(Length::Fill)
                        .align_items(Alignment::Start)
                        .push(
                            Column::new().padding(10).spacing(10).push(
                                Text::new(format!("Render at : {} fps", self.text.to_string()))
                                    .size(28),
                            ),
                        )
                        // .push(slider::Slider::new(
                        //        &mut self.sliderstate,
                        //        0.0..=100.0,
                        //        self.background_color.g,
                        //        move |b| {},
                        //    )),
                        .push(
                            button::Button::new(Text::new("Press me!"))
                                .on_press(Message::ButtonPressed),
                        ),
                ),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }
}
