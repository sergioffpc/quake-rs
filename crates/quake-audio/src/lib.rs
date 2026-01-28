pub struct AudioManager {
    manager: kira::AudioManager<kira::DefaultBackend>,
}

impl AudioManager {
    pub fn new() -> anyhow::Result<Self> {
        let manager =
            kira::AudioManager::<kira::DefaultBackend>::new(kira::AudioManagerSettings::default())?;
        Ok(Self { manager })
    }
}
