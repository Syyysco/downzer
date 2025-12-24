use dialoguer::{theme::ColorfulTheme, Select, Input, Confirm};
use crate::core::downzer::Config;
use crate::audio::sound::{get_available_sounds, validate_custom_sound};
use anyhow::Result;
use std::path::PathBuf;

pub fn show_config_panel(config: &mut Config) -> Result<bool> {
    loop {
        let options = vec![
            "🔊 Enable/Disable Sound",
            "⏱️  Sound Minimum Duration",
            "🔉 Sound Volume",
            "✅ Sound on Task Complete",
            "🎯 Sound on All Complete",
            "🎵 Change Completion Sound",
            "💾 Save and Exit",
            "❌ Exit without Saving",
        ];
        
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("⚙️  Configuration Panel")
            .items(&options)
            .default(0)
            .interact()?;
        
        match selection {
            0 => {
                config.sound_enabled = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enable sound notifications?")
                    .default(config.sound_enabled)
                    .interact()?;
                println!("✓ Sound {}", if config.sound_enabled { "enabled" } else { "disabled" });
            }
            1 => {
                config.sound_min_duration = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Minimum task duration for sound (seconds)")
                    .default(config.sound_min_duration)
                    .interact()?;
                println!("✓ Minimum duration set to {} seconds", config.sound_min_duration);
            }
            2 => {
                let volume: f32 = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Sound volume (0.0 - 1.0)")
                    .default(config.sound_volume)
                    .validate_with(|input: &f32| -> Result<(), &str> {
                        if *input >= 0.0 && *input <= 1.0 {
                            Ok(())
                        } else {
                            Err("Volume must be between 0.0 and 1.0")
                        }
                    })
                    .interact()?;
                config.sound_volume = volume;
                println!("✓ Volume set to {:.0}%", volume * 100.0);
            }
            3 => {
                config.sound_on_task_complete = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Play sound on each task completion?")
                    .default(config.sound_on_task_complete)
                    .interact()?;
                println!("✓ Task completion sound {}", 
                    if config.sound_on_task_complete { "enabled" } else { "disabled" });
            }
            4 => {
                config.sound_on_all_complete = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Play sound when all tasks complete?")
                    .default(config.sound_on_all_complete)
                    .interact()?;
                println!("✓ All tasks completion sound {}", 
                    if config.sound_on_all_complete { "enabled" } else { "disabled" });
            }
            5 => {
                if let Err(e) = change_sound(config) {
                    println!("❌ Error: {}", e);
                }
            }
            6 => {
                println!("💾 Saving configuration...");
                return Ok(true);
            }
            7 => {
                println!("❌ Discarding changes...");
                return Ok(false);
            }
            _ => {}
        }
        
        println!(); // Línea en blanco para separar
    }
}

fn change_sound(config: &mut Config) -> Result<()> {
    let sound_options = get_available_sounds();
    let mut display_options: Vec<String> = sound_options.iter().cloned().collect();
    display_options.push("📁 Load Custom Sound File...".to_string());
    display_options.push("🔙 Back".to_string());
    
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("🎵 Select completion sound")
        .items(&display_options)
        .default(0)
        .interact()?;
    
    if selection == display_options.len() - 1 {
        // Back
        return Ok(());
    }
    
    if selection == display_options.len() - 2 {
        // Custom sound
        return load_custom_sound(config);
    }
    
    // Sonido predefinido
    if selection < sound_options.len() {
        config.sound_type = sound_options[selection].clone();
        println!("✓ Sound changed to: {}", sound_options[selection]);
    }
    
    Ok(())
}

fn load_custom_sound(config: &mut Config) -> Result<()> {
    println!("\n📁 Enter the path to your custom sound file:");
    println!("   Supported formats: MP3, WAV, OGG, FLAC");
    println!("   Example: /home/user/sounds/mysound.mp3");
    println!();
    
    let path_str: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("File path")
        .interact_text()?;
    
    let path = PathBuf::from(&path_str);
    
    // Validar el archivo
    match validate_custom_sound(&path) {
        Ok(_) => {
            config.sound_type = path_str;
            println!("✓ Custom sound loaded: {}", path.display());
            Ok(())
        }
        Err(e) => {
            println!("❌ Invalid sound file: {}", e);
            Err(e)
        }
    }
}