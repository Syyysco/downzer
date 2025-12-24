use std::path::Path;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum SoundType {
    Woodensaw,
    ChatMessage,
    Tutick,
    Click,
    Tap,
    Tap2,
    Coin,
    Stepsand,
    Glass,
    Signal,
    Complete,
    Thuddry,
}

pub fn get_available_sounds() -> Vec<String> {
    vec![
        "woodensaw".to_string(),
        "chatmessage".to_string(),
        "tutick".to_string(),
        "click".to_string(),
        "tap".to_string(),
        "tap2".to_string(),
        "coin".to_string(),
        "stepsand".to_string(),
        "glass".to_string(),
        "signal".to_string(),
        "complete".to_string(),
        "thuddry".to_string(),
    ]
}

pub fn validate_custom_sound(path: &Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("Sound file not found: {:?}", path);
    }
    
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    match ext.to_lowercase().as_str() {
        "wav" | "mp3" | "ogg" | "flac" | "m4a" => Ok(()),
        _ => anyhow::bail!("Unsupported audio format: {}", ext),
    }
}

pub fn play_sound(
    _sound_type: SoundType,
    _volume: f32,
) -> Result<()> {
    // Placeholder: La reproducción de audio se implementaría con rodio
    // Por ahora solo es un stub
    Ok(())
}

pub fn play_custom_sound(
    _path: &Path,
    _volume: f32,
) -> Result<()> {
    // Placeholder: La reproducción de audio personalizado
    Ok(())
}