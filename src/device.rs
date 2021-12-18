use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use crate::{
    buttplug::{DeviceFeature, DeviceInteraction},
    funscript::{Funscript, Funscripts},
    link_file::{
        Animation, DDEvent, EquipmentState, EquipmentType, PositionChanged, SexlabEvent,
        VibrationStart,
    },
    BodyPart, EventType, GameState,
};
use buttplug::client::ButtplugClientDevice;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;
use tracing::{debug, error};

fn log_err<T, Err: std::fmt::Display>(r: Result<T, Err>) {
    if let Err(r) = r {
        error!("{}", r)
    }
}

#[derive(Debug, Clone)]
pub enum ConfigMessage {
    Complete(Config),
    Change(ConfigChange),
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(transparent)]
pub struct Config {
    map: HashMap<BodyPart, HashMap<EventType, HashMap<String, HashSet<DeviceFeature>>>>,
}

impl Config {
    pub fn should_handle(
        &self,
        device: &String,
        feature: &DeviceFeature,
        body_part: &BodyPart,
        event_type: &EventType,
    ) -> bool {
        if let Some(body_part) = self.map.get(&body_part) {
            if let Some(devices) = body_part.get(&event_type) {
                if let Some(event_type) = devices.get(device) {
                    if event_type.contains(&feature) {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn set_should_handle(
        &mut self,
        device: String,
        feature: DeviceFeature,
        body_part: BodyPart,
        event_type: EventType,
        should_handle: bool,
    ) {
        if let Some(event_types) = self.map.get_mut(&body_part) {
            if let Some(devices) = event_types.get_mut(&event_type) {
                if let Some(features) = devices.get_mut(&device) {
                    if should_handle {
                        features.insert(feature);
                    } else {
                        features.remove(&feature);
                    }
                } else {
                    if should_handle {
                        let mut features = HashSet::new();
                        features.insert(feature);
                        devices.insert(device, features);
                    }
                }
            } else {
                if should_handle {
                    let mut devices = HashMap::new();
                    let mut features = HashSet::new();
                    features.insert(feature);
                    devices.insert(device, features);
                    event_types.insert(event_type, devices);
                }
            }
        } else {
            if should_handle {
                let mut event_types = HashMap::new();
                let mut devices = HashMap::new();
                let mut features = HashSet::new();
                features.insert(feature);
                devices.insert(device, features);
                event_types.insert(event_type, devices);
                self.map.insert(body_part, event_types);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigChange {
    pub device: String,
    pub feature: DeviceFeature,
    pub body_part: BodyPart,
    pub event_type: EventType,
    pub should_handle: bool,
}

#[derive(Debug)]
struct SexlabAnimation {
    start_time: Instant,
    name: String,
    stage: u8,
    position: u8,
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

impl std::fmt::Display for Strength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Strength::VeryWeak => write!(f, "veryweak"),
            Strength::Weak => write!(f, "weak"),
            Strength::Standard => write!(f, "standard"),
            Strength::Strong => write!(f, "strong"),
            Strength::VeryStrong => write!(f, "verystrong"),
        }
    }
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
struct DDEquipmentEvent {
    ty: EquipmentType,
    time: Option<Instant>,
}

impl DDEquipmentEvent {
    fn new(ty: EquipmentType, time: Option<Instant>) -> DDEquipmentEvent {
        Self { ty, time }
    }

    fn fill_events(
        &self,
        equipped_event_name: String,
        unequipped_event_name: Option<String>,
        now: Instant,
        state: &State,
        mut next_wakeup: &mut Option<Instant>,
        mut device_values: &mut HashMap<String, HashMap<DeviceInteraction, HashMap<u32, Vec<u8>>>>,
    ) {
        if let Some(time) = self.time {
            let event_name = if self.ty != EquipmentType::None {
                Some(equipped_event_name)
            } else {
                unequipped_event_name
            };

            if let Some(event_name) = event_name {
                let anim_duration = now - time;
                let body_parts = state
                    .funscripts
                    .get_mod_event(&"devious devices".to_string(), &event_name);

                get_device_values(
                    &state,
                    body_parts,
                    anim_duration,
                    time,
                    &mut next_wakeup,
                    &mut device_values,
                );
            }
        }
    }
}

#[derive(Debug, Default)]
struct DDEquipmentEvents {
    anal: DDEquipmentEvent,
    vaginal: DDEquipmentEvent,
    nipple_piercing: DDEquipmentEvent,
    vaginal_piercing: DDEquipmentEvent,
}

impl DDEquipmentEvents {
    fn fill_events(
        &self,
        equipped_event_name: &str,
        unequipped_event_name: Option<&str>,
        now: Instant,
        state: &State,
        next_wakeup: &mut Option<Instant>,
        device_values: &mut HashMap<String, HashMap<DeviceInteraction, HashMap<u32, Vec<u8>>>>,
    ) {
        self.anal.fill_events(
            format!("{} {}", equipped_event_name, "anal"),
            unequipped_event_name.map(|name| format!("{} {}", name, "anal")),
            now,
            state,
            next_wakeup,
            device_values,
        );
        self.vaginal.fill_events(
            format!("{} {}", equipped_event_name, "vaginal"),
            unequipped_event_name.map(|name| format!("{} {}", name, "vaginal")),
            now,
            state,
            next_wakeup,
            device_values,
        );
        self.nipple_piercing.fill_events(
            format!("{} {}", equipped_event_name, "nipplepiercing"),
            unequipped_event_name.map(|name| format!("{} {}", name, "nipplepiercing")),
            now,
            state,
            next_wakeup,
            device_values,
        );
        self.vaginal_piercing.fill_events(
            format!("{} {}", equipped_event_name, "vaginalpiercing"),
            unequipped_event_name.map(|name| format!("{} {}", name, "vaginalpiercing")),
            now,
            state,
            next_wakeup,
            device_values,
        );
    }
}

#[derive(Debug)]
pub struct FunscriptInstance {
    name: String,
    start: Instant,
}

impl FunscriptInstance {
    pub fn new(name: &String, start: Instant) -> Self {
        Self {
            name: name.clone(),
            start,
        }
    }
}

#[derive(Debug, Default)]
struct State {
    devices: HashMap<String, (InteractionMap, Arc<ButtplugClientDevice>)>,
    buttplug_connected: bool,
    config: Config,
    mod_events: HashMap<u32, FunscriptInstance>,
    sexlab_animation: Option<SexlabAnimation>,
    orgasm: Option<Instant>,
    game_state: GameState,
    funscripts: Funscripts,
    dd_equip_events: DDEquipmentEvents,
    dd_step_event: DDEquipmentEvents,
    dd_vibrate_event: Option<DDVibrate>,
}

impl State {
    fn handle_message(&mut self, message: crate::Message) -> bool {
        match message {
            crate::Message::ButtplugIn(::buttplug::client::ButtplugClientEvent::DeviceAdded(
                device,
            )) => {
                let name = device.name.clone();
                self.devices.insert(name, (Default::default(), device));
                true
            }
            crate::Message::ButtplugIn(::buttplug::client::ButtplugClientEvent::DeviceRemoved(
                device,
            )) => {
                self.devices.remove(&device.name);
                true
            }
            crate::Message::ButtplugIn(::buttplug::client::ButtplugClientEvent::ServerConnect) => {
                self.devices.clear();
                self.buttplug_connected = true;
                true
            }
            crate::Message::ButtplugIn(
                ::buttplug::client::ButtplugClientEvent::ServerDisconnect,
            ) => {
                self.devices.clear();
                self.buttplug_connected = false;
                true
            }
            crate::Message::ButtplugIn(_) => false,
            crate::Message::DeviceConfiguration(ConfigMessage::Complete(config)) => {
                self.config = config;
                true
            }
            crate::Message::DeviceConfiguration(ConfigMessage::Change(ConfigChange {
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
            crate::Message::LinkFileIn(crate::link_file::InMessage::FileEvent(
                crate::link_file::Event::Game(crate::link_file::GameEvent::DamageEvent(
                    _damage_event,
                )),
            )) => {
                //todo
                false
            }
            crate::Message::LinkFileIn(crate::link_file::InMessage::FileEvent(
                crate::link_file::Event::DD(DDEvent::DeviceEvent(_davice_event)),
            )) => {
                //todo
                false
            }
            crate::Message::LinkFileIn(crate::link_file::InMessage::FileEvent(
                crate::link_file::Event::DD(DDEvent::Edged(_)),
            )) => {
                // todo
                false
            }
            crate::Message::LinkFileIn(crate::link_file::InMessage::FileEvent(
                crate::link_file::Event::DD(DDEvent::EquipmentChanged(EquipmentState {
                    vaginal,
                    anal,
                    vaginal_piercing,
                    nipple_piercing,
                })),
            )) => {
                let now = Instant::now();
                let mut changed = false;
                if self.dd_equip_events.anal.ty != anal {
                    self.dd_equip_events.anal = DDEquipmentEvent::new(anal, Some(now));
                    changed = true;
                }
                if self.dd_equip_events.anal.ty != vaginal {
                    self.dd_equip_events.vaginal = DDEquipmentEvent::new(vaginal, Some(now));
                    changed = true;
                }
                if self.dd_equip_events.vaginal_piercing.ty != vaginal_piercing {
                    self.dd_equip_events.vaginal_piercing =
                        DDEquipmentEvent::new(vaginal_piercing, Some(now));
                    changed = true;
                }
                if self.dd_equip_events.nipple_piercing.ty != nipple_piercing {
                    self.dd_equip_events.nipple_piercing =
                        DDEquipmentEvent::new(nipple_piercing, Some(now));
                    changed = true;
                }

                changed
            }
            crate::Message::LinkFileIn(crate::link_file::InMessage::FileEvent(
                crate::link_file::Event::DD(DDEvent::Orgasm(_orgasm)),
            )) => {
                //todo
                false
            }
            crate::Message::LinkFileIn(crate::link_file::InMessage::FileEvent(
                crate::link_file::Event::DD(DDEvent::VibrationStart(VibrationStart { arg })),
            )) => {
                self.dd_vibrate_event = Some(DDVibrate {
                    start_time: Instant::now(),
                    strength: Strength::from_arg(arg),
                });
                true
            }
            crate::Message::LinkFileIn(crate::link_file::InMessage::FileEvent(
                crate::link_file::Event::DD(DDEvent::VibrationStop(_)),
            )) => {
                self.dd_vibrate_event = None;
                true
            }
            crate::Message::LinkFileIn(crate::link_file::InMessage::FileEvent(
                crate::link_file::Event::Sexlab(SexlabEvent::AnimationChanged(Animation {
                    name,
                    stage,
                    position,
                    ..
                })),
            )) => {
                self.sexlab_animation = Some(SexlabAnimation {
                    start_time: Instant::now(),
                    name,
                    position,
                    stage,
                });
                debug!("animation changed");
                true
            }
            crate::Message::LinkFileIn(crate::link_file::InMessage::FileEvent(
                crate::link_file::Event::Sexlab(SexlabEvent::AnimationEnded),
            )) => {
                self.sexlab_animation = None;
                true
            }
            crate::Message::LinkFileIn(crate::link_file::InMessage::FileEvent(
                crate::link_file::Event::Sexlab(SexlabEvent::AnimationStarted(Animation {
                    name,
                    stage,
                    position,
                    ..
                })),
            )) => {
                self.sexlab_animation = Some(SexlabAnimation {
                    start_time: Instant::now(),
                    name,
                    position,
                    stage,
                });
                debug!("animation started");
                true
            }
            crate::Message::LinkFileIn(crate::link_file::InMessage::FileEvent(
                crate::link_file::Event::Sexlab(SexlabEvent::OrgasmEnded),
            )) => {
                self.orgasm = None;
                true
            }
            crate::Message::LinkFileIn(crate::link_file::InMessage::FileEvent(
                crate::link_file::Event::Sexlab(SexlabEvent::OrgasmStarted),
            )) => {
                self.orgasm = Some(Instant::now());
                true
            }
            crate::Message::LinkFileIn(crate::link_file::InMessage::FileEvent(
                crate::link_file::Event::Sexlab(SexlabEvent::PositionChanged(PositionChanged {
                    name,
                    position,
                })),
            )) => {
                if let Some(animation) = &mut self.sexlab_animation {
                    animation.start_time = Instant::now();
                    animation.name = name;
                    animation.position = position;
                    true
                } else {
                    false
                }
            }
            crate::Message::LinkFileIn(crate::link_file::InMessage::FileEvent(
                crate::link_file::Event::Sexlab(SexlabEvent::StageStarted(Animation {
                    name,
                    stage,
                    ..
                })),
            )) => {
                if let Some(animation) = &mut self.sexlab_animation {
                    animation.start_time = Instant::now();
                    animation.name = name;
                    animation.stage = stage;
                    true
                } else {
                    false
                }
            }
            crate::Message::LinkFileIn(crate::link_file::InMessage::FileEvent(
                crate::link_file::Event::Custom(event),
            )) => match event {
                crate::link_file::CustomEvent::Start(crate::link_file::CustomEventStart {
                    id,
                    ty,
                }) => {
                    self.mod_events
                        .insert(id, FunscriptInstance::new(&ty, Instant::now()));

                    true
                }
                crate::link_file::CustomEvent::Stop(crate::link_file::CustomEventStop { id }) => {
                    self.mod_events.remove(&id).is_some()
                }
            },
            crate::Message::LinkFileIn(_) => false,
            crate::Message::ProcessMessage(crate::process::Message::AnimationsChanged(
                animations,
            )) => {
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
                let now = Instant::now();

                self.dd_step_event.anal =
                    DDEquipmentEvent::new(self.dd_equip_events.anal.ty, Some(now));
                self.dd_step_event.nipple_piercing =
                    DDEquipmentEvent::new(self.dd_equip_events.nipple_piercing.ty, Some(now));
                self.dd_step_event.vaginal =
                    DDEquipmentEvent::new(self.dd_equip_events.vaginal.ty, Some(now));
                self.dd_step_event.vaginal_piercing =
                    DDEquipmentEvent::new(self.dd_equip_events.vaginal_piercing.ty, Some(now));

                change
            }
            crate::Message::ProcessMessage(crate::process::Message::GameStateChanged(
                game_state,
            )) => {
                self.game_state = game_state;
                true
            }
            crate::Message::ProcessMessage(crate::process::Message::TimerReset) => {
                if let Some(sexlab_animation) = &mut self.sexlab_animation {
                    sexlab_animation.start_time = Instant::now();
                }
                false
            }
            crate::Message::FunscriptLoaded(funscripts) => {
                self.funscripts = funscripts;
                true
            }
            crate::Message::ButtplugOut(_) => false,
            crate::Message::LinkFileOut(_) => false,
            crate::Message::ConnectToProcess(_) => false,
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
                        for (name, features) in event_type_config {
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

                            for feature in features {
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

pub async fn run(mut receiver: tokio::sync::broadcast::Receiver<crate::Message>) {
    let state = Arc::new(futures::lock::Mutex::new(State::default()));
    let wakeup = Arc::new(tokio::sync::Notify::new());

    let _handle = tokio::spawn({
        let state = state.clone();
        let wakeup = wakeup.clone();
        async move {
            while let Ok(message) = receiver.recv().await {
                let mut state = state.lock().await;
                debug!("Received Message");

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
                let stage = animation.stage + 1;
                let position = animation.position + 1;

                let body_parts = state
                    .funscripts
                    .get_sexlab_animation(&animation.name.to_lowercase(), &stage, &position)
                    .or_else(|| {
                        state.funscripts.get_sexlab_animation(
                            &"generic".to_string(),
                            &stage,
                            &position,
                        )
                    });

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

            state.dd_equip_events.fill_events(
                "dd device equiped",
                Some("dd device de-equiped"),
                now,
                &state,
                &mut next_wakeup,
                &mut device_values,
            );
            state.dd_step_event.fill_events(
                "dd device footstep",
                None,
                now,
                &state,
                &mut next_wakeup,
                &mut device_values,
            );

            if let Some(vibrate) = &state.dd_vibrate_event {
                let anim_duration = now - vibrate.start_time;
                let body_parts = state.funscripts.get_mod_event(
                    &"devious devices".to_string(),
                    &format!("vibrator_{}1LP", vibrate.strength),
                );

                get_device_values(
                    &state,
                    body_parts,
                    anim_duration,
                    vibrate.start_time,
                    &mut next_wakeup,
                    &mut device_values,
                );
            }

            for (_id, FunscriptInstance { start, name }) in &state.mod_events {
                let anim_duration = now - *start;
                let body_parts = state.funscripts.get_mod_event(&"custom".to_string(), &name);

                get_device_values(
                    &state,
                    body_parts,
                    anim_duration,
                    *start,
                    &mut next_wakeup,
                    &mut device_values,
                );
            }

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
