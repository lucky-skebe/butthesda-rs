use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
#[serde(tag = "mod")]
pub enum Event {
    #[serde(rename = "game")]
    Game(GameEvent),
    #[serde(rename = "sla")]
    Sla(SLAEvent),

    #[serde(rename = "dd")]
    DD(DDEvent),

    #[serde(rename = "sexlab")]
    Sexlab(SexlabEvent),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "event")]
pub enum GameEvent {
    #[serde(rename = "menu opened")]
    MenuOpened,
    #[serde(rename = "menu closed")]
    MenuClosed,
    #[serde(rename = "loading save done")]
    LoadingSaveDone,
    #[serde(rename = "loading save")]
    LoadingSave(LoadingSaveEvent),
    #[serde(rename = "damage")]
    DamageEvent(DamageEvent),
}

#[derive(Debug, Deserialize)]
pub struct LoadingSaveEvent {
    #[serde(rename = "DD_Running")]
    dd_running: bool,
    #[serde(rename = "SGO_Running")]
    sgo_running: bool,
    #[serde(rename = "BF_Running")]
    bf_running: bool,
    #[serde(rename = "MME_Running")]
    mme_running: bool,
    #[serde(rename = "SLA_Running")]
    sla_running: bool,
}

#[derive(Debug, Deserialize)]
pub struct DamageEvent {
    source: String,
    projectile: String,
    #[serde(rename="powerAttack")]
    power_attack: bool,
    blocked: bool,
}

#[derive(Debug, Deserialize)]
pub struct SLAEvent {
    arousal: u8,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "event")]
pub enum DDEvent {
    #[serde(rename = "(de)equiped")]
    EquipmentChanged(EquipmentChanged),
    #[serde(rename = "vibrate effect start")]
    VibrationStart(VibrationStart),
    #[serde(rename = "vibrate effect stop")]
    VibrationStop(VibrationStop),
    #[serde(rename = "orgasm")]
    Orgasm(Orgasm),
    #[serde(rename = "edged")]
    Edged(Edged),
    #[serde(rename = "device event")]
    DeviceEvent(DeviceEvent),
}

#[derive(Debug, Deserialize)]
pub struct EquipmentChanged {
    vaginal: EquipmentType,
    anal: EquipmentType,
    #[serde(rename = "vaginalPiecing")]
    vaginal_piercing: EquipmentType,
    #[serde(rename = "nipplePiercing")]
    nipple_piercing: EquipmentType,
}

#[derive(Debug, Deserialize)]
pub enum EquipmentType {
    None,
    Pump,
    Magic,
    Normal,
}

#[derive(Debug, Deserialize)]
pub struct VibrationStart {
    arg: f32,
}

#[derive(Debug, Deserialize)]
pub struct VibrationStop {
    arg: f32,
}

#[derive(Debug, Deserialize)]
pub struct Orgasm {
    arg: f32,
}

#[derive(Debug, Deserialize)]
pub struct Edged {
    arg: f32,
}

#[derive(Debug, Deserialize)]
pub enum DeviceEvent {
    #[serde(rename = "trip over")]
    TripOver,
    #[serde(rename = "drip")]
    Drip,
    #[serde(rename = "stamina drain")]
    StaminaDrain,
    #[serde(rename = "blindfold mystery")]
    BlindfoldMystery,
    #[serde(rename = "restraints+armor")]
    RestraintsAndArmor,
    #[serde(rename = "posture collar")]
    PostureCollar,
    #[serde(rename = "wet padding")]
    WetPadding,
    #[serde(rename = "blindold trip")]
    BlindfoldTrip,
    #[serde(rename = "nipple piercings")]
    NupplePiercings,
    #[serde(rename = "tight corset")]
    TightCorset,
    #[serde(rename = "plug moan")]
    PlugMoan,
    #[serde(rename = "trip and fall")]
    TripAndFall,
    #[serde(rename = "bump pumps")]
    BumpPumps,
    #[serde(rename = "struggle")]
    Struggle,
    #[serde(rename = "belted empty")]
    BeltedEmpty,
    #[serde(rename = "mounted")]
    Mounted,
    #[serde(rename = "tight gloves")]
    TightGloves,
    #[serde(rename = "bra chafing")]
    BraChafing,
    #[serde(rename = "periodic shock")]
    PeriodicShock,
    #[serde(rename = "arm cuff fumble")]
    ArmCuffFumble,
    #[serde(rename = "draugr plug vibration")]
    DraugnPlugVibration,
    #[serde(rename = "restrictive collar")]
    RestictiveCollar,
    #[serde(rename = "mana drain")]
    ManaDrain,
    #[serde(rename = "vibration")]
    Vibration,
    #[serde(rename = "harness")]
    Harness,
    #[serde(rename = "horny")]
    Horny,
    #[serde(rename = "chaos plug")]
    ChaosPlug,
    #[serde(rename = "belt chafing")]
    BeltChafing,
    #[serde(rename = "health drain")]
    HealthDrain,
    #[serde(rename = "organicvibrationeffect")]
    OrganicVibrationEffect,
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "event")]
pub enum SexlabEvent {
    #[serde(rename = "animation started")]
    AnimationStarted(Animation),
    #[serde(rename = "animation changed")]
    AnimationChanged(Animation),
    #[serde(rename = "animation ended")]
    AnimationEnded,
    #[serde(rename = "stage started")]
    StageStarted(StageStarted),
    #[serde(rename = "stage ended")]
    StageEnded,
    #[serde(rename = "position changed")]
    PositionChanged(PositionChanged),
    #[serde(rename = "orgasm started")]
    OrgasmStarted,
    #[serde(rename = "orgasm ended")]
    OrgasmEnded,
}

#[derive(Debug, Deserialize)]
pub struct Animation {
    name: String,
    stage: u8,
    #[serde(rename = "pos")]
    position: u8,
    #[serde(rename = "usingStrappon")]
    using_strapon: bool,
    #[serde(rename = "isMale")]
    is_male: bool,
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct StageStarted {
    name: String,
    stage: u8,
}

#[derive(Debug, Deserialize)]
pub struct PositionChanged {
    name: String,
    #[serde(rename = "pos")]
    position: u8,
}
