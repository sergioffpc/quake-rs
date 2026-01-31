use crate::EventWriter;
use crate::component::{Dirty, EntityId, Transform};
use crate::world::{EntityEvent, WorldEvent};
use legion::{IntoQuery, system};
use quake_asset::pak::dem::{Dem, DemEvent};
use quake_audio::{AudioEvent, SoundId};
use std::collections::VecDeque;
use std::time::Duration;
use tracing::debug;

pub struct DemPlayback {
    events: VecDeque<DemEvent>,
    due_events: VecDeque<DemEvent>,
    cursor: usize,
    duration: Duration,
}

impl DemPlayback {
    pub fn new(dem: Dem) -> Self {
        let events = dem.iter().collect::<VecDeque<_>>();
        Self {
            events,
            due_events: Default::default(),
            cursor: 0,
            duration: Duration::from_secs(0),
        }
    }

    pub fn advance(&mut self, delta_time: Duration) {
        self.due_events.clear();
        if self.cursor >= self.events.len() {
            return;
        }

        while let Some(event) = self.events.get(self.cursor) {
            if let DemEvent::Time { time } = event {
                if self.duration < Duration::from_secs_f32(*time) {
                    break;
                } else {
                    self.cursor += 1;
                    continue;
                }
            }

            self.due_events.push_back(event.clone());
            self.cursor += 1;
        }

        self.duration += delta_time;
    }

    pub fn iter(&self) -> impl Iterator<Item = &DemEvent> {
        self.due_events.iter()
    }
}

#[system]
pub fn dem_advance_playback(
    #[resource] delta_time: &Duration,
    #[resource] playback: &mut DemPlayback,
) {
    playback.advance(*delta_time);
}

#[system]
#[read_component(EntityId)]
#[read_component(Transform)]
#[write_component(Transform)]
pub fn dem_process_playback(
    world: &mut legion::world::SubWorld,
    command_buffer: &mut legion::systems::CommandBuffer,
    #[resource] playback: &DemPlayback,
    #[resource] event_writer: &mut EventWriter,
) {
    for event in playback.iter() {
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
                        handle_index: *entity_id,
                        volume: *volume,
                        attenuation: *attenuation,
                        sound_id: SoundId::from(*sound_id),
                        position: transform.position,
                    }));
                }
            }
            // stop a sound on entity/channel.
            DemEvent::StopSound { entity_id, channel } => {
                event_writer.push(WorldEvent::Audio(AudioEvent::Stop {
                    handle_index: *entity_id,
                }));
            }
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
            DemEvent::UpdateEntity {
                entity_id,
                model_id,
                frame_id,
                colormap,
                skin_id,
                effects,
                origin1,
                origin2,
                origin3,
                angle1,
                angle2,
                angle3,
                no_lerp,
            } => {
                let entity_id = EntityId::from(*entity_id);
                let mut query = <(legion::Entity, &EntityId, &mut Transform)>::query();
                if let Some((entity, _, transform)) =
                    query.iter_mut(world).find(|(_, id, _)| **id == entity_id)
                {
                    fn set_if_some<T: Copy>(opt: &Option<T>, set: impl FnOnce(T)) {
                        if let Some(value) = opt.as_ref() {
                            set(*value);
                        }
                    }
                    set_if_some(origin1, |v| transform.position.x = v);
                    set_if_some(origin2, |v| transform.position.y = v);
                    set_if_some(origin3, |v| transform.position.z = v);

                    command_buffer.add_component(*entity, Dirty);
                }
            } // full client state (view height, punch, velocity, items, weapon, ammo, etc.).
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
                command_buffer.push((entity_id, transform, Dirty));

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
