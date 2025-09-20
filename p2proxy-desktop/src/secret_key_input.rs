use crate::{App, AppMessage, HasKeyState};
use iced::Task;
use iced_core::Padding;
use p2proxy_client::init_endpoint;
use p2proxy_lib::display_chain;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub(super) enum SecretKeyMessage {
    WantSecretKeyFile,
    GenerateSecretKey,
    SecretFilePickResult(SecretFilePickResult),
    SecretKeyHexInput(String),
    SecretKeyHexSubmit,
}

#[derive(Debug, Clone)]
pub(super) enum SecretFilePickResult {
    NoFilePicked,
    LoadKeyError(String),
    LoadedState(Box<HasKeyState>),
}

impl App {
    pub(super) fn on_secret_key_message(
        &mut self,
        secret_key_message: SecretKeyMessage,
    ) -> Task<AppMessage> {
        match secret_key_message {
            SecretKeyMessage::WantSecretKeyFile => {
                return Task::future(async move {
                    let Some(file) = rfd::AsyncFileDialog::new().pick_file().await else {
                        return AppMessage::SecretKeyForm(SecretKeyMessage::SecretFilePickResult(
                            SecretFilePickResult::NoFilePicked,
                        ));
                    };
                    match p2proxy_client::load_secret_key(file.path()) {
                        Ok(o) => match p2proxy_client::init_endpoint(o.clone()).await {
                            Ok(ep) => {
                                AppMessage::SecretKeyForm(SecretKeyMessage::SecretFilePickResult(
                                    SecretFilePickResult::LoadedState(
                                        HasKeyState::new_from_secret_key_and_ep(o, ep),
                                    ),
                                ))
                            }
                            Err(e) => {
                                AppMessage::SecretKeyForm(SecretKeyMessage::SecretFilePickResult(
                                    SecretFilePickResult::LoadKeyError(
                                        display_chain(&*e).to_string(),
                                    ),
                                ))
                            }
                        },
                        Err(e) => {
                            AppMessage::SecretKeyForm(SecretKeyMessage::SecretFilePickResult(
                                SecretFilePickResult::LoadKeyError(display_chain(&*e).to_string()),
                            ))
                        }
                    }
                });
            }
            SecretKeyMessage::SecretFilePickResult(result) => match result {
                SecretFilePickResult::NoFilePicked => {
                    self.setup_state.secret_key_input_error = Some("No file picked".to_string());
                }
                SecretFilePickResult::LoadKeyError(e) => {
                    self.setup_state.secret_key_input_error = Some(e);
                }
                SecretFilePickResult::LoadedState(k) => {
                    self.has_key_state = Some(k);
                }
            },
            SecretKeyMessage::SecretKeyHexInput(i) => {
                self.setup_state.secret_key_input = i;
            }
            SecretKeyMessage::SecretKeyHexSubmit => {
                let input = self.setup_state.secret_key_input.clone();
                return Task::future(async move {
                    match iroh_base::SecretKey::from_str(&input) {
                        Ok(sk) => match init_endpoint(sk.clone()).await {
                            Ok(ep) => {
                                AppMessage::SecretKeyForm(SecretKeyMessage::SecretFilePickResult(
                                    SecretFilePickResult::LoadedState(
                                        HasKeyState::new_from_secret_key_and_ep(sk, ep),
                                    ),
                                ))
                            }
                            Err(e) => {
                                AppMessage::SecretKeyForm(SecretKeyMessage::SecretFilePickResult(
                                    SecretFilePickResult::LoadKeyError(
                                        display_chain(&*e).to_string(),
                                    ),
                                ))
                            }
                        },
                        Err(e) => {
                            AppMessage::SecretKeyForm(SecretKeyMessage::SecretFilePickResult(
                                SecretFilePickResult::LoadKeyError(display_chain(&e).to_string()),
                            ))
                        }
                    }
                });
            }
            SecretKeyMessage::GenerateSecretKey => {
                let key = p2proxy_client::generate_secret_key();
                self.setup_state.secret_key_input = hex::encode(key.to_bytes());
                return Task::future(async move {
                    match p2proxy_client::init_endpoint(key.clone()).await {
                        Ok(ep) => {
                            AppMessage::SecretKeyForm(SecretKeyMessage::SecretFilePickResult(
                                SecretFilePickResult::LoadedState(
                                    HasKeyState::new_from_secret_key_and_ep(key, ep),
                                ),
                            ))
                        }
                        Err(e) => {
                            AppMessage::SecretKeyForm(SecretKeyMessage::SecretFilePickResult(
                                SecretFilePickResult::LoadKeyError(display_chain(&*e).to_string()),
                            ))
                        }
                    }
                });
            }
        }
        Task::none()
    }
    pub(super) fn secret_key_input<'a>(&self) -> iced::widget::Column<'a, AppMessage> {
        const SPACE: f32 = 5.0;
        let column = iced::widget::column([iced::widget::row([
            iced::widget::text_input("secret key hex", &self.setup_state.secret_key_input)
                .on_input(|ip| AppMessage::SecretKeyForm(SecretKeyMessage::SecretKeyHexInput(ip)))
                .into(),
            iced::widget::horizontal_space().width(SPACE).into(),
            iced::widget::button("submit")
                .on_press(AppMessage::SecretKeyForm(
                    SecretKeyMessage::SecretKeyHexSubmit,
                ))
                .into(),
            iced::widget::horizontal_space().width(SPACE).into(),
            iced::widget::button("Choose file")
                .on_press(AppMessage::SecretKeyForm(
                    SecretKeyMessage::WantSecretKeyFile,
                ))
                .into(),
            iced::widget::horizontal_space().width(SPACE).into(),
            iced::widget::button("Generate key")
                .on_press(AppMessage::SecretKeyForm(
                    SecretKeyMessage::GenerateSecretKey,
                ))
                .into(),
        ])
        .padding(Padding::default().top(10.))
        .into()]);
        if let Some(e) = &self.setup_state.secret_key_input_error {
            column.push(iced::widget::text(format!("Error: {e}")))
        } else {
            column
        }
    }
}
