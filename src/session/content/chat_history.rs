use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use tdgrand::{enums, functions, types};

use crate::session::{content::MessageRow, Chat};
use crate::RUNTIME;

mod imp {
    use super::*;
    use adw::subclass::prelude::BinImpl;
    use gtk::CompositeTemplate;
    use once_cell::sync::Lazy;
    use std::cell::{Cell, RefCell};

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/melix99/telegrand/ui/content-chat-history.ui")]
    pub struct ChatHistory {
        pub compact: Cell<bool>,
        pub chat: RefCell<Option<Chat>>,
        #[template_child]
        pub history_list_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub message_entry: TemplateChild<gtk::TextView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ChatHistory {
        const NAME: &'static str = "ContentChatHistory";
        type Type = super::ChatHistory;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            MessageRow::static_type();
            Self::bind_template(klass);

            klass.install_action("history.send-message", None, move |widget, _, _| {
                widget.send_message();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ChatHistory {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpec::new_boolean(
                        "compact",
                        "Compact",
                        "Wheter a compact view is used or not",
                        false,
                        glib::ParamFlags::READWRITE,
                    ),
                    glib::ParamSpec::new_object(
                        "chat",
                        "Chat",
                        "The chat currently shown",
                        Chat::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "compact" => {
                    let compact = value.get().unwrap();
                    self.compact.set(compact);
                }
                "chat" => {
                    let chat = value.get().unwrap();
                    obj.set_chat(chat);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "compact" => self.compact.get().to_value(),
                "chat" => obj.chat().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
        }
    }

    impl WidgetImpl for ChatHistory {}
    impl BinImpl for ChatHistory {}
}

glib::wrapper! {
    pub struct ChatHistory(ObjectSubclass<imp::ChatHistory>)
        @extends gtk::Widget, adw::Bin;
}

impl ChatHistory {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create ChatHistory")
    }

    fn send_message(&self) {
        if let Some(chat) = self.chat() {
            let self_ = imp::ChatHistory::from_instance(self);
            let buffer = self_.message_entry.buffer();
            let text = types::FormattedText {
                text: buffer
                    .text(&buffer.start_iter(), &buffer.end_iter(), true)
                    .to_string(),
                ..Default::default()
            };
            let content = types::InputMessageText {
                text,
                clear_draft: true,
                ..Default::default()
            };
            let message = enums::InputMessageContent::InputMessageText(content);

            let client_id = chat.session().client_id();
            let chat_id = chat.id();

            RUNTIME.spawn(async move {
                functions::SendMessage::new()
                    .chat_id(chat_id)
                    .input_message_content(message)
                    .send(client_id)
                    .await
                    .unwrap();
            });

            buffer.set_text("");
        }
    }

    fn save_draft_message(&self) {
        if let Some(chat) = self.chat() {
            let self_ = imp::ChatHistory::from_instance(self);
            let buffer = self_.message_entry.buffer();
            let draft = buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), true)
                .to_string();

            if chat.draft_message() != draft {
                let text = types::FormattedText {
                    text: draft,
                    ..Default::default()
                };
                let content = types::InputMessageText {
                    text,
                    ..Default::default()
                };
                let draft_message = types::DraftMessage {
                    input_message_text: enums::InputMessageContent::InputMessageText(content),
                    ..Default::default()
                };

                let client_id = chat.session().client_id();
                let chat_id = chat.id();

                RUNTIME.spawn(async move {
                    functions::SetChatDraftMessage::new()
                        .chat_id(chat_id)
                        .draft_message(draft_message)
                        .send(client_id)
                        .await
                        .unwrap();
                });
            }
        }
    }

    pub fn chat(&self) -> Option<Chat> {
        let self_ = imp::ChatHistory::from_instance(self);
        self_.chat.borrow().clone()
    }

    pub fn set_chat(&self, chat: Option<Chat>) {
        if self.chat() == chat {
            return;
        }

        self.save_draft_message();

        let self_ = imp::ChatHistory::from_instance(self);
        if let Some(ref chat) = chat {
            self_.message_entry.buffer().set_text(&chat.draft_message());

            chat.history().fetch();

            let selection = gtk::NoSelection::new(Some(&chat.history()));
            self_.history_list_view.set_model(Some(&selection));
        }

        self_.chat.replace(chat);
        self.notify("chat");
    }
}
