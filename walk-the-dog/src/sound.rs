use anyhow::{anyhow, Result};
use js_sys::ArrayBuffer;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{AudioBuffer, AudioBufferSourceNode, AudioContext, AudioNode};

use crate::browser;

pub fn create_audio_context() -> Result<AudioContext> {
    AudioContext::new().map_err(|err| anyhow!("Failed to create audio context: {:#?}", err))
}

fn create_buffer_source(ctx: &AudioContext) -> Result<AudioBufferSourceNode> {
    ctx.create_buffer_source()
        .map_err(|err| anyhow!("Failed to create buffer source: {:#?}", err))
}

fn connect_with_audio_node(
    source: &AudioBufferSourceNode,
    destination: &AudioNode,
) -> Result<AudioNode> {
    source
        .connect_with_audio_node(destination)
        .map_err(|err| anyhow!("Failed to connect with audio node: {:#?}", err))
}

fn create_track_source(ctx: &AudioContext, buffer: &AudioBuffer, volume: f32) -> Result<AudioBufferSourceNode> {
    let track_source = create_buffer_source(ctx)?;
    track_source.set_buffer(Some(&buffer));

    // adjust volume
    let gain_node = ctx
        .create_gain()
        .map_err(|err| anyhow!("Failed to create gain node: {:#?}", err))?;
    gain_node
        .gain()
        .set_value_at_time(volume, ctx.current_time())
        .map_err(|err| anyhow!("Failed to set gain value at time: {:#?}", err))?;

    connect_with_audio_node(&track_source, &gain_node)?;
    gain_node
        .connect_with_audio_node(&ctx.destination())
        .map_err(|err| anyhow!("Failed to set gain value at time: {:#?}", err))?;

    Ok(track_source)
}

pub enum LOOPING {
    NO,
    YES,
}

pub fn play_sound(ctx: &AudioContext, buffer: &AudioBuffer, looping: LOOPING, volume: f32) -> Result<()> {
    let source = create_track_source(ctx, buffer, volume)?;
    if matches!(looping, LOOPING::YES) {
        source.set_loop(true);
    }

    source
        .start()
        .map_err(|err| anyhow!("Failed to start sound: {:#?}", err))
}

pub async fn decode_audio_data(
    ctx: &web_sys::AudioContext,
    array_buffer: &ArrayBuffer,
) -> Result<AudioBuffer> {
    JsFuture::from(
        ctx.decode_audio_data(&array_buffer)
            .map_err(|err| anyhow!("Failed to decode audio data: {:#?}", err))?,
    )
    .await
    .map_err(|err| anyhow!("Failed to decode audio data: {:#?}", err))?
    .dyn_into()
    .map_err(|err| anyhow!("Failed to cast audio buffer: {:#?}", err))
}

#[derive(Clone)]
pub struct Audio {
    context: AudioContext,
}

#[derive(Clone)]
pub struct Sound {
    buffer: AudioBuffer,
}

impl Audio {
    pub fn new() -> Result<Self> {
        Ok(Audio {
            context: create_audio_context()?,
        })
    }

    pub async fn load_sound(&self, filename: &str) -> Result<Sound> {
        let array_buffer = browser::fetch_array_buffer(filename).await?;
        let audio_buffer = decode_audio_data(&self.context, &array_buffer).await?;

        Ok(Sound {
            buffer: audio_buffer,
        })
    }

    pub fn play_sound(&self, sound: &Sound, volume: f32) -> Result<()> {
        play_sound(&self.context, &sound.buffer, LOOPING::NO, volume)
    }

    pub fn play_looping_sound(&self, sound: &Sound, volume: f32) -> Result<()> {
        play_sound(&self.context, &sound.buffer, LOOPING::YES, volume)
    }
}
