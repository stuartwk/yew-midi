use midir::{Ignore, MidiInput};
use send_wrapper::SendWrapper;
use std::error::Error;
use std::sync::{Arc, Mutex};
use yew::prelude::*;
extern crate console_error_panic_hook;
extern crate js_sys;
extern crate midir;
extern crate wasm_bindgen;
extern crate web_sys;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Debug)]
enum Msg {
    MidiReceived(Vec<u8>),
}

struct App {
    link: ComponentLink<Self>,
    last_midi_message: Vec<u8>,
    midi_closure: Option<Closure<dyn FnMut()>>,
}

impl App {
    // pulled from the mimir example https://github.com/Boddlnagg/midir/blob/master/examples/browser/src/lib.rs
    fn run_midi(cb: SendWrapper<Callback<Vec<u8>>>) -> Result<bool, Box<dyn Error>> {
        let window = web_sys::window().expect("no global `window` exists");

        let mut midi_in = MidiInput::new("midir reading input")?;
        midi_in.ignore(Ignore::None);

        // Get an input port
        let ports = midi_in.ports();
        let in_port = match &ports[..] {
            [] => {
                log::info!("No ports available yet, will try again");
                return Ok(false);
            }
            [ref port] => {
                log::info!(
                    "Choosing the only available input port: {}",
                    midi_in.port_name(port).unwrap()
                );
                port
            }
            _ => {
                let mut msg = "Choose an available input port:\n".to_string();
                for (i, port) in ports.iter().enumerate() {
                    msg.push_str(format!("{}: {}\n", i, midi_in.port_name(port).unwrap()).as_str());
                }
                loop {
                    if let Ok(Some(port_str)) = window.prompt_with_message_and_default(&msg, "0") {
                        if let Ok(port_int) = port_str.parse::<usize>() {
                            if let Some(port) = &ports.get(port_int) {
                                break port.clone();
                            }
                        }
                    }
                }
            }
        };
        log::info!("Opening connection");
        let in_port_name = midi_in.port_name(in_port)?;

        // _conn_in needs to be a named parameter, because it needs to be kept alive until the end of the scope
        let _conn_in = midi_in.connect(
            in_port,
            "midir-read-input",
            // cb,
            move |stamp, message, _| {
                log::info!("{}: {:?} (len = {})", stamp, message, message.len());
                cb.emit(message.to_vec());
            },
            (),
        )?;

        log::info!("Connection open, reading input from '{}'", in_port_name);
        Box::leak(Box::new(_conn_in));
        Ok(true)
    }

    fn build_midi_closure(&self) -> Closure<dyn FnMut()> {
        let token_outer = Arc::new(Mutex::new(None));
        let token = token_outer.clone();

        let callback = self
            .link
            .callback(|response: Vec<u8>| Msg::MidiReceived(response));

        let wrapped_callback = SendWrapper::new(callback);

        let closure: Closure<dyn FnMut()> = Closure::wrap(Box::new(move || {
            let wrapped_callback = wrapped_callback.clone();

            if Self::run_midi(wrapped_callback).unwrap() == true {
                if let Some(token) = *token.lock().unwrap() {
                    web_sys::window().unwrap().clear_interval_with_handle(token);
                }
            }
        }));
        *token_outer.lock().unwrap() = web_sys::window()
            .unwrap()
            .set_interval_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                200,
            )
            .ok();
        closure
    }
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_dispatch: Self::Properties, link: ComponentLink<Self>) -> Self {
        let midi_vec: Vec<u8> = Vec::new();

        Self {
            link,
            last_midi_message: midi_vec,
            midi_closure: None,
        }
    }

    fn rendered(&mut self, first_render: bool) {
        if first_render {
            let closure = self.build_midi_closure();
            self.midi_closure = Some(closure);
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::MidiReceived(response) => {
                self.last_midi_message = response;
                true
            }
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        let msg = self.last_midi_message.clone();

        html! {
            <div>
                <pre>{ msg }</pre>
            </div>
        }
    }

    fn destroy(&mut self) {}
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<App>();
}
