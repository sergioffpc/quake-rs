use crate::EventWriter;
use crate::component::{EntityId, Transform};
use crate::world::{EntityEvent, WorldEvent};
use legion::{IntoQuery, system};
use quake_asset::pak::dem::{Dem, DemEvent};
use quake_audio::{AudioEvent, SoundId};
use std::collections::VecDeque;
use std::time::Duration;
use tracing::debug;

pub struct DemPlayback {
    events: VecDeque<DemEvent>,
    cursor: usize,
    duration: Duration,
}

impl DemPlayback {
    pub fn new(dem: Dem) -> Self {
        let events = dem.iter().collect::<VecDeque<_>>();
        Self {
            events,
            cursor: 0,
            duration: Duration::from_secs(0),
        }
    }

    pub fn advance(&mut self, delta_time: Duration) -> Option<DueEvents<'_>> {
        if self.cursor >= self.events.len() {
            return None;
        }

        let mut due_events = VecDeque::new();
        while let Some(event) = self.events.get(self.cursor) {
            if let DemEvent::Time { time } = event {
                if self.duration < Duration::from_secs_f32(*time) {
                    break;
                } else {
                    self.cursor += 1;
                    continue;
                }
            }

            due_events.push_back(event);
            self.cursor += 1;
        }

        self.duration += delta_time;

        Some(due_events)
    }
}

type DueEvents<'a> = VecDeque<&'a DemEvent>;

#[system]
#[read_component(EntityId)]
#[read_component(Transform)]
pub fn replay_dem_stream(
    world: &mut legion::world::SubWorld,
    command_buffer: &mut legion::systems::CommandBuffer,
    #[resource] playback: &mut DemPlayback,
    #[resource] delta_time: &Duration,
    #[resource] event_writer: &mut EventWriter,
) {
    let due_events = playback.advance(*delta_time).unwrap_or_default();
    for event in due_events.iter() {
        debug!(?event, "replay dem event");

        match event {
            // set view/camera entity.
            DemEvent::SetView { .. } => {}
            // start a sound with params at origin.
            DemEvent::PlaySound {
                volume,
                attenuation,
                entity_id,
                sound_id,
                ..
            } => {
                let mut query = <(&EntityId, &Transform)>::query();
                if let Some((_id, transform)) = query
                    .iter(world)
                    .find(|(id, _)| usize::from(**id) == *entity_id)
                {
                    event_writer.push(WorldEvent::Audio(AudioEvent::Play {
                        volume: *volume,
                        attenuation: *attenuation,
                        sound_id: SoundId::from(*sound_id),
                        position: transform.position,
                    }));
                }
            }
            // stop a sound on entity/channel.
            DemEvent::StopSound { entity_id, channel } => {}
            // console print text.
            DemEvent::Print { .. } => {}
            // server-injected console command(s).
            DemEvent::StuffText { .. } => {}
            // set view angles.
            DemEvent::SetAngle { .. } => {}
            // lightstyle index and its pattern string bytes.
            DemEvent::LightStyle { .. } => {}
            // update client stat (health, ammo, etc.).
            DemEvent::UpdateStat { .. } => {}
            // set player name.
            DemEvent::UpdateName { .. } => {}
            // update frag count.
            DemEvent::UpdateFrags { .. } => {}
            // player color settings.
            DemEvent::UpdateColors { .. } => {}
            // delta entity update (model/frame/colormap/skin/effects, origin/angles, no-lerp).
            DemEvent::UpdateEntity { .. } => {}
            // full client state (view height, punch, velocity, items, weapon, ammo, etc.).
            DemEvent::ClientData { .. } => {}
            // particle burst.
            DemEvent::Particle { .. } => {}
            // damage indicators and origin.
            DemEvent::Damage { .. } => {}
            // spawn a static entity.
            DemEvent::SpawnStatic { .. } => {}
            // baseline for an entity.
            DemEvent::SpawnBaseline {
                entity_id,
                model_id,
                frame_id,
                colormap,
                skin_id,
                origin,
                angles,
            } => {
                let entity_id = EntityId::from(*entity_id);
                let transform = Transform { position: *origin };
                command_buffer.push((entity_id, transform));

                event_writer.push(WorldEvent::Entity(EntityEvent::Spawn {
                    entity_id,
                    transform,
                }))
            }
            // temporary entity effect
            DemEvent::SpawnTemporary { .. } => {}
            // looped/static sound at origin.
            DemEvent::SpawnStaticSound { .. } => {}
            // pause/unpause.
            DemEvent::SetPause { .. } => {}
            // center-screen message.
            DemEvent::CenterPrint { .. } => {}
            // monster kill counter increment.
            DemEvent::KilledMonster => {}
            // secret found counter increment.
            DemEvent::FoundSecret => {}
            // enter intermission.
            DemEvent::Intermission => {}
            // finale message sequence.
            DemEvent::Finale { .. } => {}
            // play CD track.
            DemEvent::CdTrack { .. } => {}
            // show end-of-demo “sell” screen.
            DemEvent::SellScreen => {}
            // cutscene text.
            DemEvent::CutScene { .. } => {}
            _ => (),
        }
    }
}
