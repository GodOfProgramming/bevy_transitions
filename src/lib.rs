use bevy_app::prelude::*;
use bevy_color::prelude::*;
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_picking::Pickable;
use bevy_reflect::{Reflectable, prelude::*};
use bevy_state::{prelude::*, state::FreelyMutableState};
use bevy_time::prelude::*;
use bevy_ui::{FocusPolicy, InteractionDisabled, prelude::*};
use derive_new::new;
use std::marker::PhantomData;

pub struct TransitionsPlugin<S, C>(PhantomData<(S, C)>)
where
    S: FreelyMutableState + Reflectable,
    C: Component;

impl<S, C> Plugin for TransitionsPlugin<S, C>
where
    S: FreelyMutableState + Reflectable,
    C: Component,
{
    fn build(&self, app: &mut App) {
        app.init_resource::<TransitionSpeed>()
            .init_resource::<PendingState<S>>()
            .add_message::<TransitionMessage<S>>()
            .add_observer(Self::on_camera_change)
            .add_observer(Self::on_camera_despawn)
            .add_systems(Update, Self::apply_fade)
            .add_systems(FixedUpdate, Self::handle_transition_events);
    }
}

impl<S, C> Default for TransitionsPlugin<S, C>
where
    S: FreelyMutableState + Reflectable + Clone,
    C: Component,
{
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<S, C> TransitionsPlugin<S, C>
where
    S: FreelyMutableState + Reflectable + Clone,
    C: Component,
{
    fn apply_fade(
        mut q_overlays: Query<&mut BackgroundColor, With<FadeOverlay>>,
        mut transition: Transition<S>,
        time: Res<Time>,
    ) {
        for mut overlay in &mut q_overlays {
            let alpha =
                (overlay.0.alpha() + transition.speed() * time.delta_secs()).clamp(0.0, 1.0);
            overlay.0.set_alpha(alpha);

            if alpha >= 1.0
                && let Some(pending) = transition.take_pending()
            {
                transition.writer.write(TransitionMessage::new(pending));
                transition.set_speed(-transition.speed().abs());
            }
        }
    }

    fn handle_transition_events(
        mut events: MessageReader<TransitionMessage<S>>,
        mut next_state: ResMut<NextState<S>>,
    ) {
        for event in events.read() {
            next_state.set(event.state.clone());
        }
    }

    fn on_camera_change(event: On<Add, C>, mut commands: Commands) {
        commands.spawn((
            Name::new("Fade Overlay"),
            FadeOverlay,
            BackgroundColor(Color::linear_rgba(0.0, 0.0, 0.0, 1.0)),
            UiTargetCamera(event.event_target()),
            OverlayOf(event.event_target()),
            FocusPolicy::Pass,
            InteractionDisabled,
            Pickable::IGNORE,
            GlobalZIndex(i32::MAX),
            Node {
                position_type: PositionType::Absolute,
                top: px(0.0),
                left: px(0.0),
                width: percent(100.0),
                height: percent(100.0),
                ..Default::default()
            },
        ));
    }

    fn on_camera_despawn(
        event: On<Remove, Overlays>,
        mut commands: Commands,
        q_overlays: Query<&Overlays>,
    ) {
        let Ok(overlays) = q_overlays.get(event.event_target()) else {
            return;
        };

        for entity in &overlays.0 {
            commands.entity(*entity).despawn();
        }
    }
}

#[derive(new, Message)]
pub struct TransitionMessage<S>
where
    S: FreelyMutableState + Clone,
{
    state: S,
}

#[derive(SystemParam)]
pub struct Transition<'w, S>
where
    S: FreelyMutableState + Reflectable,
{
    writer: MessageWriter<'w, TransitionMessage<S>>,
    speed: ResMut<'w, TransitionSpeed>,
    pending_state: ResMut<'w, PendingState<S>>,
}

impl<S> Transition<'_, S>
where
    S: FreelyMutableState + Reflectable,
{
    pub fn to(&mut self, state: S) {
        self.pending_state.0 = Some(state);
        self.set_speed(self.speed().abs());
    }

    pub fn speed(&self) -> f32 {
        self.speed.0
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed.0 = speed;
    }

    fn take_pending(&mut self) -> Option<S> {
        self.pending_state.0.take()
    }
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct PendingState<S>(Option<S>)
where
    S: FreelyMutableState + Reflectable;

impl<S> Default for PendingState<S>
where
    S: FreelyMutableState + Reflectable,
{
    fn default() -> Self {
        Self(None)
    }
}

#[derive(Resource, Reflect)]
#[reflect(Resource, Default)]
pub struct TransitionSpeed(f32);

impl Default for TransitionSpeed {
    fn default() -> Self {
        Self(2.0)
    }
}
#[derive(Component)]
#[relationship_target(relationship=OverlayOf, linked_spawn)]
struct Overlays(Vec<Entity>);

#[derive(Component)]
#[relationship(relationship_target=Overlays)]
struct OverlayOf(Entity);

#[derive(Component)]
struct FadeOverlay;

pub fn is_transition_pending<S>(mut events: MessageReader<TransitionMessage<S>>) -> bool
where
    S: FreelyMutableState + Clone,
{
    let had_events = !events.is_empty();
    events.clear();
    had_events
}
