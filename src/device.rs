use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use crate::{
    buttplug::{DeviceFeature, DeviceInteraction},
    funscript::{Funscript, Funscripts},
    link_file::{
        Animation, DDEvent, DamageEvent, EquipmentState, PositionChanged, SexlabEvent,
        StageStarted, VibrationStart,
    },
    BodyPart, EventType,
};
use buttplug::client::ButtplugClientDevice;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;
use tracing::error;

fn log_err<T, Err: std::fmt::Display>(r: Result<T, Err>) {
    if let Err(r) = r {
        error!("{}", r)
    }
}

pub enum LogicMessage {
    Buttplug(ButtplugMessage),
    Config(ConfigMessage),
    File(FileMessage),
    Process(ProcessMessage),
    Funscript(crate::funscript::Funscripts),
}

pub enum ConfigMessage {
    Complete(Config),
    Change(ConfigChange),
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    map: HashMap<BodyPart, HashMap<EventType, HashSet<(String, DeviceFeature)>>>,
}

impl Config {
    fn should_handle(
        &self,
        device: String,
        feature: DeviceFeature,
        body_part: &BodyPart,
        event_type: &EventType,
    ) -> bool {
        if let Some(body_part) = self.map.get(&body_part) {
            if let Some(event_type) = body_part.get(&event_type) {
                if event_type.contains(&(device, feature)) {
                    return true;
                }
            }
        }

        false
    }

    fn set_should_handle(
        &mut self,
        device: String,
        feature: DeviceFeature,
        body_part: BodyPart,
        event_type: EventType,
        should_handle: bool,
    ) {
        if let Some(body_part) = self.map.get_mut(&body_part) {
            if let Some(event_type) = body_part.get_mut(&event_type) {
                if should_handle {
                    event_type.insert((device, feature));
                } else {
                    event_type.remove(&(device, feature));
                }
            } else {
                if should_handle {
                    let mut set = HashSet::new();
                    set.insert((device, feature));
                    body_part.insert(event_type, set);
                }
            }
        } else {
            if should_handle {
                let mut map = HashMap::new();
                let mut set = HashSet::new();
                set.insert((device, feature));
                map.insert(event_type, set);
                self.map.insert(body_part, map);
            }
        }
    }
}

pub struct ConfigChange {
    device: String,
    feature: DeviceFeature,
    body_part: BodyPart,
    event_type: EventType,
    should_handle: bool,
}

pub enum ButtplugMessage {
    Disconnected,
    Connected,
    DeviceAdded(Arc<ButtplugClientDevice>),
    DeviceRemoved(Arc<ButtplugClientDevice>),
}

pub enum FileMessage {
    DamageMessage(DamageEvent),
    DeviousDevices(DDEvent),
    SexLab(SexlabEvent),
}

pub enum ProcessMessage {
    Animation(Vec<String>),
    GamePaused,
    GameUnPaused,
    GameStarted,
    GameStopped,
    TimerReset,
}

#[derive(Debug)]
struct FunscriptInstance {
    start_time: Instant,
}

#[derive(Debug)]
struct SexlabAnimation {
    start_time: Instant,
    name: String,
    stage: u8,
    position: u8,
}

#[derive(Debug, PartialEq, Eq)]
enum GameState {
    Stopped,
    Running,
    Paused,
}

impl Default for GameState {
    fn default() -> Self {
        Self::Stopped
    }
}

#[derive(Debug, Default, PartialEq)]
struct InteractionMap {
    vibrate: Option<HashMap<u32, f64>>,
    rotate: Option<HashMap<u32, (f64, bool)>>,
    linear: Option<HashMap<u32, (f64, f32)>>,
}

#[derive(Debug)]
enum Strength {
    VeryWeak,
    Weak,
    Standard,
    Strong,
    VeryStrong,
}

impl Strength {
    fn from_arg(arg: f32) -> Self {
        if arg >= 5.0 {
            Self::VeryStrong
        } else if arg >= 4.0 {
            Self::Strong
        } else if arg >= 4.0 {
            Self::Standard
        } else if arg >= 4.0 {
            Self::Weak
        } else {
            Self::VeryWeak
        }
    }
}

#[derive(Debug)]
struct DDVibrate {
    start_time: Instant,
    strength: Strength,
}

#[derive(Debug, Default)]
struct State {
    devices: HashMap<String, (InteractionMap, Arc<ButtplugClientDevice>)>,
    buttplug_connected: bool,
    config: Config,
    // mod_events: HashMap<(String, String), FunscriptInstance>,
    sexlab_animation: Option<SexlabAnimation>,
    orgasm: bool,
    game_state: GameState,
    funscripts: Funscripts,
    dd_vibrateEvent: Option<DDVibrate>,
    dd_equipment_state: EquipmentState,
}

impl State {
    fn set_handle(
        &mut self,
        device: String,
        body_part: BodyPart,
        event_type: EventType,
        should_handle: bool,
    ) {
    }

    fn handle_message(&mut self, message: LogicMessage) -> bool {
        match message {
            LogicMessage::Buttplug(ButtplugMessage::DeviceAdded(device)) => {
                let name = device.name.clone();
                self.devices.insert(name, (Default::default(), device));
                true
            }
            LogicMessage::Buttplug(ButtplugMessage::DeviceRemoved(device)) => {
                self.devices.remove(&device.name);
                true
            }
            LogicMessage::Buttplug(ButtplugMessage::Connected) => {
                self.devices.clear();
                self.buttplug_connected = true;
                true
            }
            LogicMessage::Buttplug(ButtplugMessage::Disconnected) => {
                self.devices.clear();
                self.buttplug_connected = false;
                true
            }
            LogicMessage::Config(ConfigMessage::Complete(config)) => {
                self.config = config;
                true
            }
            LogicMessage::Config(ConfigMessage::Change(ConfigChange {
                device,
                feature,
                body_part,
                event_type,
                should_handle,
            })) => {
                self.config.set_should_handle(
                    device,
                    feature,
                    body_part,
                    event_type,
                    should_handle,
                );
                true
            }
            LogicMessage::File(FileMessage::DamageMessage(_damage_event)) => {
                //todo
                false
            }
            LogicMessage::File(FileMessage::DeviousDevices(DDEvent::DeviceEvent(
                _davice_event,
            ))) => {
                //todo
                false
            }
            LogicMessage::File(FileMessage::DeviousDevices(DDEvent::Edged(_))) => {
                self.dd_vibrateEvent = None;
                true
            }
            LogicMessage::File(FileMessage::DeviousDevices(DDEvent::EquipmentChanged(
                EquipmentState {
                    vaginal,
                    anal,
                    vaginal_piercing,
                    nipple_piercing,
                },
            ))) => false,
            LogicMessage::File(FileMessage::DeviousDevices(DDEvent::Orgasm(_orgasm))) => {
                //todo
                false
            }
            LogicMessage::File(FileMessage::DeviousDevices(DDEvent::VibrationStart(
                VibrationStart { arg },
            ))) => {
                self.dd_vibrateEvent = Some(DDVibrate {
                    start_time: Instant::now(),
                    strength: Strength::from_arg(arg),
                });
                true
            }
            LogicMessage::File(FileMessage::DeviousDevices(DDEvent::VibrationStop(_))) => {
                self.dd_vibrateEvent = None;
                true
            }
            LogicMessage::File(FileMessage::SexLab(SexlabEvent::AnimationChanged(Animation {
                name,
                stage,
                position,
                ..
            }))) => {
                self.sexlab_animation = Some(SexlabAnimation {
                    start_time: Instant::now(),
                    name,
                    position,
                    stage,
                });
                true
            }
            LogicMessage::File(FileMessage::SexLab(SexlabEvent::AnimationEnded)) => {
                self.sexlab_animation = None;
                true
            }
            LogicMessage::File(FileMessage::SexLab(SexlabEvent::AnimationStarted(Animation {
                name,
                stage,
                position,
                ..
            }))) => {
                self.sexlab_animation = Some(SexlabAnimation {
                    start_time: Instant::now(),
                    name,
                    position,
                    stage,
                });
                true
            }
            LogicMessage::File(FileMessage::SexLab(SexlabEvent::OrgasmEnded)) => {
                self.orgasm = false;
                true
            }
            LogicMessage::File(FileMessage::SexLab(SexlabEvent::OrgasmStarted)) => {
                self.orgasm = true;
                true
            }
            LogicMessage::File(FileMessage::SexLab(SexlabEvent::PositionChanged(
                PositionChanged { name, position },
            ))) => {
                if let Some(animation) = &mut self.sexlab_animation {
                    animation.name = name;
                    animation.position = position;
                    true
                } else {
                    false
                }
            }
            LogicMessage::File(FileMessage::SexLab(SexlabEvent::StageEnded)) => false,
            LogicMessage::File(FileMessage::SexLab(SexlabEvent::StageStarted(StageStarted {
                name,
                stage,
            }))) => {
                if let Some(animation) = &mut self.sexlab_animation {
                    animation.name = name;
                    animation.stage = stage;
                    true
                } else {
                    false
                }
            }
            LogicMessage::Process(ProcessMessage::Animation(animations)) => {
                let mut change = false;
                for a in animations {
                    change |= match a.as_str() {
                        "FootRight"
                        | "FootLeft"
                        | "JumpUp"
                        | "JumpDown"
                        | "IdleChairSitting"
                        | "idleChairGetUp"
                        | "tailCombatIdle"
                        | "tailSneakIdle"
                        | "IdleStop"
                        | "weaponSwing"
                        | "weaponLeftSwing"
                        | "tailMTLocomotion"
                        | "tailSneakLocomotion"
                        | "tailCombatLocomotion" => true,
                        _ => false,
                    }
                }
                change
            }
            LogicMessage::Process(ProcessMessage::GamePaused) => {
                self.game_state = GameState::Paused;
                true
            }
            LogicMessage::Process(ProcessMessage::GameStarted) => {
                self.game_state = GameState::Running;
                true
            }
            LogicMessage::Process(ProcessMessage::GameStopped) => {
                self.game_state = GameState::Stopped;
                true
            }
            LogicMessage::Process(ProcessMessage::GameUnPaused) => {
                self.game_state = GameState::Running;
                true
            }
            LogicMessage::Process(ProcessMessage::TimerReset) => {
                if let Some(sexlab_animation) = &mut self.sexlab_animation {
                    sexlab_animation.start_time = Instant::now();
                }
                false
            }
            LogicMessage::Funscript(funscripts) => {
                self.funscripts = funscripts;
                true
            }
        }
    }
}

fn get_device_values(
    state: &State,
    body_parts: Option<&HashMap<BodyPart, HashMap<EventType, Funscript>>>,
    anim_duration: Duration,
    start_time: Instant,
    next_wakeup: &mut Option<Instant>,
    device_values: &mut HashMap<String, HashMap<DeviceInteraction, HashMap<u32, Vec<u8>>>>,
) {
    if let Some(body_parts) = body_parts {
        for (body_part, event_types) in body_parts {
            if let Some(body_part_config) = state.config.map.get(body_part) {
                for (event_type, script) in event_types {
                    if let Some(event_type_config) = body_part_config.get(event_type) {
                        for (name, feature) in event_type_config {
                            let (value, next_update) = script.get_action_at(anim_duration);

                            if let Some(next_update) = next_update {
                                let possible_wakeup = start_time + next_update;
                                if let Some(wakeup) = next_wakeup.as_mut() {
                                    if *wakeup < possible_wakeup {
                                        *wakeup = possible_wakeup;
                                    }
                                } else {
                                    *next_wakeup = Some(possible_wakeup);
                                }
                            }

                            insert_into(
                                device_values,
                                name.clone(),
                                feature.interaction.clone(),
                                feature.index.clone(),
                                value.unwrap_or_default(),
                            );
                        }
                    }
                }
            }
        }
    }
}

fn insert_into(
    device_values: &mut HashMap<String, HashMap<DeviceInteraction, HashMap<u32, Vec<u8>>>>,
    name: String,
    interaction: DeviceInteraction,
    index: u32,
    value: u8,
) {
    if let Some(interactions) = device_values.get_mut(&name) {
        if let Some(instances) = interactions.get_mut(&interaction) {
            if let Some(values) = instances.get_mut(&index) {
                values.push(value);
            } else {
                let mut values = Vec::new();
                values.push(value);
                instances.insert(index, values);
            }
        } else {
            let mut instances = HashMap::new();
            let mut values = Vec::new();
            values.push(value);
            instances.insert(index, values);
            interactions.insert(interaction, instances);
        }
    } else {
        let mut interactions = HashMap::new();
        let mut instances = HashMap::new();
        let mut values = Vec::new();
        values.push(value);
        instances.insert(index, values);
        interactions.insert(interaction, instances);
        device_values.insert(name, interactions);
    }
}

pub async fn run(mut receiver: tokio::sync::mpsc::Receiver<LogicMessage>) {
    let state = Arc::new(futures::lock::Mutex::new(State::default()));
    let wakeup = Arc::new(tokio::sync::Notify::new());

    let _handle = tokio::spawn({
        let state = state.clone();
        let wakeup = wakeup.clone();
        async move {
            while let Some(message) = receiver.recv().await {
                let mut state = state.lock().await;

                if state.handle_message(message) {
                    wakeup.notify_one();
                }
            }
        }
    });

    let mut next_wakeup = Some(tokio::time::Instant::now());
    let mut running = false;

    loop {
        match next_wakeup.take() {
            Some(next_wakeup) => {
                tokio::select! {
                    _ = tokio::time::sleep_until(next_wakeup) => {}
                    _ = wakeup.notified() => {}
                }
            }
            None => wakeup.notified().await,
        }

        {
            let now = Instant::now();

            let mut state = state.lock().await;
            if !state.buttplug_connected || state.game_state == GameState::Stopped {
                continue;
            }

            if state.game_state == GameState::Paused && running {
                running = false;
                for (_, device) in state.devices.values() {
                    device.stop();
                }
            }

            if state.game_state == GameState::Running && !running {
                for (features, device) in state.devices.values() {
                    if let Some(ref values) = features.vibrate {
                        log_err(
                            device
                                .vibrate(buttplug::client::VibrateCommand::SpeedMap(values.clone()))
                                .await,
                        );
                    }
                    if let Some(ref values) = features.rotate {
                        log_err(
                            device
                                .rotate(buttplug::client::RotateCommand::RotateMap(values.clone()))
                                .await,
                        );
                    }
                }
            }

            let mut device_values: HashMap<
                String,
                HashMap<DeviceInteraction, HashMap<u32, Vec<u8>>>,
            > = HashMap::new();

            state.sexlab_animation.as_ref().map(|animation| {
                let body_parts = state.funscripts.get_sexlab_animation(
                    &animation.name,
                    &animation.stage,
                    &animation.position,
                );

                let anim_duration = now - animation.start_time;

                get_device_values(
                    &state,
                    body_parts,
                    anim_duration,
                    animation.start_time,
                    &mut next_wakeup,
                    &mut device_values,
                );
            });

            // for ((mod_name, event_name), f) in &state.mod_events {
            //     let anim_duration = now - f.start_time;
            //     let body_parts = state.funscripts.get_mod_event(mod_name, event_name);

            //     get_device_values(
            //         &state,
            //         body_parts,
            //         anim_duration,
            //         f.start_time,
            //         &mut next_wakeup,
            //         &mut device_values,
            //     );
            // }

            for (device_name, features) in device_values {
                let mut new_map = InteractionMap {
                    ..Default::default()
                };
                for (interaction, instances) in features {
                    for (index, values) in instances {
                        let count = values.len() as f64;

                        let new_value = 1f64.min(
                            values
                                .into_iter()
                                .map(|v| (v as f64).powf(count))
                                .sum::<f64>()
                                .powf(1.0 / count),
                        );

                        match interaction {
                            DeviceInteraction::Vibrate => {
                                if let Some(vibrate) = new_map.vibrate.as_mut() {
                                    vibrate.insert(index, new_value);
                                } else {
                                    let mut vibrate = HashMap::new();
                                    vibrate.insert(index, new_value);
                                    new_map.vibrate = Some(vibrate);
                                }
                            }
                            DeviceInteraction::Rotate => {
                                if let Some(rotate) = new_map.rotate.as_mut() {
                                    rotate.insert(index, (new_value, true));
                                } else {
                                    let mut rotate = HashMap::new();
                                    rotate.insert(index, (new_value, true));
                                    new_map.rotate = Some(rotate);
                                }
                            }
                        }
                    }
                }

                if let Some((map, device)) = state.devices.get_mut(&device_name) {
                    if new_map != *map {
                        if let Some(ref values) = new_map.vibrate {
                            log_err(
                                device
                                    .vibrate(buttplug::client::VibrateCommand::SpeedMap(
                                        values.clone(),
                                    ))
                                    .await,
                            );
                        }
                        if let Some(ref values) = new_map.rotate {
                            log_err(
                                device
                                    .rotate(buttplug::client::RotateCommand::RotateMap(
                                        values.clone(),
                                    ))
                                    .await,
                            );
                        }

                        *map = new_map
                    }
                }
            }
        }
    }
}
