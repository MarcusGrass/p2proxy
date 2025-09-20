mod observability;
mod peer_id;
mod peer_node_state;
mod secret_key_input;

use crate::peer_id::{PeerId, PeerIdGenerator};
use crate::peer_node_state::{PeerNode, PeerNodeState, PeerNodeStateMessage};
use crate::secret_key_input::SecretKeyMessage;
use iced::{Element, Task};
use iced_core::{Padding, Theme};
use iroh::Endpoint;

fn main() {
    observability::setup_observability();
    iced::application(App::default, App::update, App::view)
        .executor::<tokio::runtime::Runtime>()
        .title("P2Proxy Desktop")
        .decorations(true)
        .theme(App::theme)
        .run()
        .unwrap();
}

#[derive(Debug, Clone)]
enum AppMessage {
    SecretKeyForm(SecretKeyMessage),
    WipeState,
    PeerNodeState(PeerNodeStateMessage),
}

#[derive(Debug, Clone)]
struct KeyPair {
    #[allow(dead_code)]
    private: iroh_base::SecretKey,
    public: iroh_base::PublicKey,
}

#[derive(Default)]
struct App {
    peer_id_generator: PeerIdGenerator,
    setup_state: SetupState,
    has_key_state: Option<Box<HasKeyState>>,
}

#[derive(Default)]
struct SetupState {
    secret_key_input: String,
    secret_key_input_error: Option<String>,
}

#[derive(Debug, Clone)]
struct HasKeyState {
    key_pair: KeyPair,
    endpoint: Endpoint,
    peer_node_state: PeerNodeState,
}

impl HasKeyState {
    pub fn peer_mut_by_id(&mut self, peer_id: PeerId) -> Option<&mut PeerNode> {
        self.peer_node_state
            .peers
            .iter_mut()
            .find(|p| p.peer_id == peer_id)
    }
    pub fn new_from_secret_key_and_ep(key: iroh_base::SecretKey, endpoint: Endpoint) -> Box<Self> {
        Box::new(Self {
            key_pair: KeyPair {
                public: key.public(),
                private: key,
            },
            endpoint,
            peer_node_state: PeerNodeState::default(),
        })
    }
}

impl App {
    fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        match message {
            AppMessage::SecretKeyForm(skm) => self.on_secret_key_message(skm),
            AppMessage::WipeState => {
                self.has_key_state.take();
                self.setup_state = SetupState::default();
                Task::none()
            }
            AppMessage::PeerNodeState(pnsm) => self.on_peer_node_state_message(pnsm),
        }
    }

    #[inline]
    #[allow(clippy::unused_self)]
    fn theme(&self) -> Theme {
        Theme::CatppuccinMocha
    }

    fn view(&self) -> Element<'_, AppMessage> {
        let column = iced::widget::Column::new()
            .push(self.secret_key_input())
            .padding(Padding::new(10.).vertical());

        let Some(has_key) = &self.has_key_state else {
            return column.into();
        };
        // The app has a parseable key
        column
            .push(iced::widget::Space::with_height(20.))
            .push(iced::widget::row([
                iced::widget::text("node id:").line_height(1.85).into(),
                iced::widget::horizontal_space().width(20.).into(),
                iced::widget::text_input("", &has_key.key_pair.public.to_string()).into(),
                iced::widget::horizontal_space().width(10.).into(),
                iced::widget::button("Clear")
                    .on_press(AppMessage::WipeState)
                    .style(iced::widget::button::danger)
                    .into(),
            ]))
            .push(iced::widget::Space::with_height(20.))
            .push(self.peer_node_state())
            .into()
    }
}
