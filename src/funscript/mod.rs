use std::{collections::HashMap, path::Path};

use tokio::io::AsyncReadExt;
use tracing::error;

mod contracts;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BodyPart {
    Head,
    Body,
    Breast,
    Belly,
    Feet,
    Mouth,
    Vaginal,
    Clit,
    Anal,
}

impl BodyPart {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "head" => Some(Self::Head),
            "body" => Some(Self::Body),
            "breast" => Some(Self::Breast),
            "belly" => Some(Self::Belly),
            "feet" => Some(Self::Feet),
            "mouth" => Some(Self::Mouth),
            "vaginal" => Some(Self::Vaginal),
            "clit" => Some(Self::Clit),
            "anal" => Some(Self::Anal),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    Shock,
    Damage,
    Penetrate,
    Vibrate,
    Equip,
}

impl EventType {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "shock" => Some(Self::Shock),
            "damage" => Some(Self::Damage),
            "penetrate" => Some(Self::Penetrate),
            "vibrate" => Some(Self::Vibrate),
            "equip" => Some(Self::Equip),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Funscripts {
    sexlab: HashMap<
        String,
        HashMap<
            String,
            HashMap<u8, HashMap<u8, HashMap<BodyPart, HashMap<EventType, contracts::Funscript>>>>,
        >,
    >,
    mod_events: HashMap<
        String,
        HashMap<String, HashMap<BodyPart, HashMap<EventType, contracts::Funscript>>>,
    >,
}

impl Funscripts {
    pub fn get_mod_event(
        &self,
        mod_name: String,
        event_name: String,
    ) -> Option<&HashMap<BodyPart, HashMap<EventType, contracts::Funscript>>> {
        self.mod_events.get(&mod_name)?.get(&event_name)
    }

    pub fn get_sexlab_animation(
        &self,
        animation_pack: String,
        animation_name: String,
        stage: u8,
        position: u8,
    ) -> Option<&HashMap<BodyPart, HashMap<EventType, contracts::Funscript>>> {
        self.sexlab
            .get(&animation_pack)?
            .get(&animation_name)?
            .get(&stage)?
            .get(&position)
    }

    async fn load_funscript(path: impl AsRef<Path>) -> Result<contracts::Funscript, anyhow::Error> {
        let mut file = tokio::fs::File::open(path).await?;
        let mut s = String::new();
        file.read_to_string(&mut s).await?;
        let script = serde_json::from_str(&s)?;

        Ok(script)
    }

    async fn load_event_types(
        path: impl AsRef<Path>,
    ) -> Result<HashMap<EventType, contracts::Funscript>, anyhow::Error> {
        let mut event_type_map = HashMap::new();
        let mut read_dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            if entry.file_type().await?.is_file() {
                let file_name = entry.file_name();
                let file_name = file_name.to_str();
                let path = entry.path();

                if let Some(file_name) = file_name {
                    if let Some((name, _extension)) = file_name.rsplit_once(".") {
                        if let Some(event_type) = EventType::from_str(&name.to_lowercase()) {
                            event_type_map
                                .insert(event_type, Self::load_funscript(path).await?);
                        } else {
                            error!(?path, "Invalid EventType: {}", file_name);
                        }
                    }
                }
            }
        }
        Ok(event_type_map)
    }

    async fn load_body_parts(
        path: impl AsRef<Path>,
    ) -> Result<HashMap<BodyPart, HashMap<EventType, contracts::Funscript>>, anyhow::Error> {
        let mut body_part_map = HashMap::new();
        let mut read_dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let file_name = entry.file_name();
                let file_name = file_name.to_str();
                if let Some(file_name) = file_name {
                    if let Some(body_part) = BodyPart::from_str(&file_name.to_lowercase()) {
                        body_part_map
                            .insert(body_part, Self::load_event_types(entry.path()).await?);
                    } else {
                        let path = entry.path();
                        error!(?path, "Invalid BodyPart: {}", file_name);
                    }
                }
            }
        }
        Ok(body_part_map)
    }

    async fn load_mod_events(
        path: impl AsRef<Path>,
    ) -> Result<
        HashMap<String, HashMap<BodyPart, HashMap<EventType, contracts::Funscript>>>,
        anyhow::Error,
    > {
        let mut mod_map = HashMap::new();
        let mut read_dir = tokio::fs::read_dir(path).await?;
        while let Some(event_entry) = read_dir.next_entry().await? {
            if event_entry.file_type().await?.is_dir() {
                let file_name = event_entry.file_name();
                let file_name = file_name.to_str();
                if let Some(file_name) = file_name {
                    mod_map.insert(
                        file_name.to_string().to_lowercase(),
                        Self::load_body_parts(event_entry.path()).await?,
                    );
                }
            }
        }
        Ok(mod_map)
    }

    async fn load_stage(
        path: impl AsRef<Path>,
    ) -> Result<
        HashMap<u8, HashMap<BodyPart, HashMap<EventType, contracts::Funscript>>>,
        anyhow::Error,
    > {
        let mut map = HashMap::new();
        let mut read_dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let file_name = entry.file_name();
                let file_name = file_name.to_str();
                if let Some(file_name) = file_name {
                    let file_name = file_name.to_string().to_lowercase();
                    if file_name.starts_with("p") {
                        if let Some(position) = file_name[1..].parse::<u8>().ok() {
                            map.insert(position, Self::load_body_parts(entry.path()).await?);
                        }else {
                            let path = entry.path();
                            error!(?path, "Invalid Sexlab Position: {}", file_name);
                        }
                    }else {
                        let path = entry.path();
                        error!(?path, "Invalid Sexlab Position: {}", file_name);
                    }
                }
            }
        }
        Ok(map)
    }

    async fn load_animation(
        path: impl AsRef<Path>,
    ) -> Result<
        HashMap<u8, HashMap<u8, HashMap<BodyPart, HashMap<EventType, contracts::Funscript>>>>,
        anyhow::Error,
    > {
        let mut map = HashMap::new();
        let mut read_dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let file_name = entry.file_name();
                let file_name = file_name.to_str();
                if let Some(file_name) = file_name {
                    let file_name = file_name.to_string().to_lowercase();
                    if file_name.starts_with("s") {
                        if let Some(stage) = file_name[1..].parse::<u8>().ok() {
                            map.insert(stage, Self::load_stage(entry.path()).await?);
                        }else {
                            let path = entry.path();
                            error!(?path, "Invalid Sexlab Stage: {}", file_name);
                        }
                    }else {
                        let path = entry.path();
                        error!(?path, "Invalid Sexlab Stage: {}", file_name);
                    }
                }
            }
        }
        Ok(map)
    }

    async fn load_anim_pack(
        path: impl AsRef<Path>,
    ) -> Result<
        HashMap<
            String,
            HashMap<u8, HashMap<u8, HashMap<BodyPart, HashMap<EventType, contracts::Funscript>>>>,
        >,
        anyhow::Error,
    > {
        let mut map = HashMap::new();
        let mut read_dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let file_name = entry.file_name();
                let file_name = file_name.to_str();
                if let Some(file_name) = file_name {
                    map.insert(
                        file_name.to_string().to_lowercase(),
                        Self::load_animation(entry.path()).await?,
                    );
                }
            }
        }
        Ok(map)
    }

    pub async fn load(path: impl AsRef<Path>) -> Result<Self, anyhow::Error> {
        let mut sexlab = HashMap::new();
        let mut mod_events = HashMap::new();
        let mut read_dir = tokio::fs::read_dir(path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let file_name = entry.file_name();
                let file_name = file_name.to_str();
                match file_name {
                    Some(mod_name) if mod_name.to_lowercase() == "sexlab" => {
                        let mut read_dir = tokio::fs::read_dir(entry.path()).await?;
                        while let Some(entry) = read_dir.next_entry().await? {
                            if entry.file_type().await?.is_dir() {
                                let file_name = entry.file_name();
                                let file_name = file_name.to_str();
                                match file_name {
                                    Some(orgasm_name) if orgasm_name.to_lowercase() == "orgasm" => {
                                        let body_parts =
                                            Self::load_body_parts(entry.path()).await?;
                                        let mut sexlab_map = HashMap::new();

                                        sexlab_map.insert("orgasm".to_string(), body_parts);

                                        mod_events.insert("sexlab".to_string(), sexlab_map);
                                    }
                                    Some(anim_pack_name) => {
                                        sexlab.insert(
                                            anim_pack_name.to_string().to_lowercase(),
                                            Self::load_anim_pack(entry.path()).await?,
                                        );
                                    }
                                    None => {}
                                }
                            }
                        }
                    }
                    Some(mod_name) => {
                        mod_events.insert(
                            mod_name.to_string().to_lowercase(),
                            Self::load_mod_events(entry.path()).await?,
                        );
                    }
                    None => {}
                }
            }
        }

        Ok(Self { mod_events, sexlab })
    }
}
