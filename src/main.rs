use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    render::camera::Camera,
    render::camera::ScalingMode,
};

use std::f32::consts::TAU;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
#[system_set(base)]
pub enum CameraSystem {
    CameraRigMovement,
    CameraRigFollow,
}

pub struct FourXCameraPlugin;

impl Plugin for FourXCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(camera_rig_movement.in_base_set(CameraSystem::CameraRigMovement))
            .add_system(
                camera_rig_follow
                    .in_base_set(CameraSystem::CameraRigFollow)
                    .after(CameraSystem::CameraRigMovement),
            );
    }
}

pub struct KeyboardConf {
    pub forward: Box<[KeyCode]>,
    pub backward: Box<[KeyCode]>,
    pub left: Box<[KeyCode]>,
    pub right: Box<[KeyCode]>,
    /// sensitivity is calculated by mx + c where (m: f32, c: f32)
    /// and x is the camera distance
    pub move_sensitivity: (f32, f32),
    pub clockwise: Box<[KeyCode]>,
    pub counter_clockwise: Box<[KeyCode]>,
    pub rotate_sensitivity: f32,
}

impl Default for KeyboardConf {
    fn default() -> Self {
        KeyboardConf {
            forward: Box::new([KeyCode::W, KeyCode::Up]),
            backward: Box::new([KeyCode::S, KeyCode::Down]),
            left: Box::new([KeyCode::A, KeyCode::Left]),
            right: Box::new([KeyCode::D, KeyCode::Right]),
            move_sensitivity: (2.0, 0.1),
            clockwise: Box::new([KeyCode::Q]),
            counter_clockwise: Box::new([KeyCode::E]),
            rotate_sensitivity: std::f32::consts::PI / 10.,
        }
    }
}

pub struct MouseConf {
    pub rotate: MouseButton,
    pub rotate_sensitivity: f32,
    pub drag: MouseButton,
    /// sensitivity is calculated by mx + c where (m: f32, c: f32)
    /// and x is the camera distance
    pub drag_sensitivity: (f32, f32),
    pub zoom_sensitivity: f32,
}

impl Default for MouseConf {
    fn default() -> Self {
        MouseConf {
            rotate: MouseButton::Right,
            rotate_sensitivity: std::f32::consts::PI / 1000.,
            drag: MouseButton::Left,
            drag_sensitivity: (1., std::f32::consts::PI / 1000.),
            zoom_sensitivity: 1.,
        }
    }
}

/// TODO: Add the ability set more input type here like gamepad
#[derive(Component, Default)]
pub struct CameraRig {
    pub keyboard: KeyboardConf,
    pub mouse: MouseConf,
    // Transforms for (Rig, Camera)
    pub move_to: (Option<Transform>, Option<Transform>),
    pub disable: bool,
}

#[derive(Bundle, Default)]
pub struct CameraRigBundle {
    pub camera_rig: CameraRig,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

fn camera_rig_movement(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mouse_input: Res<Input<MouseButton>>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut camera_rig_query: Query<(&mut CameraRig, &Children, Entity)>,
    mut rig_cam_query: ParamSet<(
        Query<&mut Transform, With<CameraRig>>,
        Query<&mut Transform, With<Camera>>,
    )>,
    mut follow_query: Query<&mut CameraRigFollow>,
) {
    for (mut rig, children, entity) in camera_rig_query.iter_mut() {
        if rig.disable {
            continue;
        }

        let mut rig_transform = if let Ok(transform) = rig_cam_query.p0().get_mut(entity) {
            transform.clone()
        } else {
            panic!("Rig missing a transform")
        };

        let mut move_to_rig = if let Some(trans) = rig.move_to.0 {
            trans
        } else {
            rig_transform
        };

        let mut translated = false;
        let move_sensitivity = rig_transform.translation.y * rig.keyboard.move_sensitivity.0
            + rig.keyboard.move_sensitivity.1;
        // Rig Keyboard Movement
        if rig
            .keyboard
            .forward
            .iter()
            .any(|key| keyboard_input.pressed(*key))
        {
            move_to_rig.translation += rig_transform.rotation * Vec3::X * move_sensitivity;
            translated = true;
        }
        if rig
            .keyboard
            .backward
            .iter()
            .any(|key| keyboard_input.pressed(*key))
        {
            move_to_rig.translation -= rig_transform.rotation * Vec3::X * move_sensitivity;
            translated = true;
        }
        if rig
            .keyboard
            .right
            .iter()
            .any(|key| keyboard_input.pressed(*key))
        {
            move_to_rig.translation += rig_transform.rotation * Vec3::Z * move_sensitivity;
            translated = true;
        }
        if rig
            .keyboard
            .left
            .iter()
            .any(|key| keyboard_input.pressed(*key))
        {
            move_to_rig.translation -= rig_transform.rotation * Vec3::Z * move_sensitivity;
            translated = true;
        }

        // Rig Keyboard Rotation
        if rig
            .keyboard
            .counter_clockwise
            .iter()
            .any(|key| keyboard_input.pressed(*key))
        {
            move_to_rig.rotate(Quat::from_rotation_y(rig.keyboard.rotate_sensitivity));
        }
        if rig
            .keyboard
            .clockwise
            .iter()
            .any(|key| keyboard_input.pressed(*key))
        {
            move_to_rig.rotate(Quat::from_rotation_y(-rig.keyboard.rotate_sensitivity));
        }

        // Rig Mouse Motion
        let mut mouse_delta_y = 0.;
        for event in mouse_motion_events.iter() {
            if mouse_input.pressed(rig.mouse.rotate) {
                move_to_rig.rotate(Quat::from_rotation_y(
                    -rig.mouse.rotate_sensitivity * event.delta.x,
                ));
                mouse_delta_y += event.delta.y;
            }
            if mouse_input.pressed(rig.mouse.drag) {
                let drag_sensitivity = rig_transform.translation.y * rig.mouse.drag_sensitivity.0
                    + rig.mouse.drag_sensitivity.1;
                move_to_rig.translation += rig_transform.rotation
                    * Vec3::new(event.delta.y, 0., -event.delta.x)
                    * drag_sensitivity;
                translated = true;
            }
        }

        if translated {
            for mut followable in follow_query.iter_mut() {
                followable.0 = false;
            }
        }

        rig.move_to.0 = Some(move_to_rig);

        // Smoothly move the rig
        if move_to_rig.translation != rig_transform.translation {
            if move_to_rig
                .translation
                .distance(rig_transform.translation)
                .abs()
                > 0.005
            {
                rig_transform.translation = rig_transform.translation.lerp(
                    move_to_rig.translation,
                    time.delta().as_micros() as f32 / 100000.,
                );
            } else {
                rig_transform.translation = move_to_rig.translation;
            }
        }
        if move_to_rig.rotation != rig_transform.rotation {
            if !move_to_rig
                .rotation
                .abs_diff_eq(rig_transform.rotation, 0.00001)
            {
                rig_transform.rotation = rig_transform.rotation.lerp(
                    move_to_rig.rotation,
                    time.delta().as_micros() as f32 / 100000.,
                );
            } else {
                rig_transform.rotation = move_to_rig.rotation;
            }
        }
        for child in children.iter() {
            if let Ok(mut transform) = rig_cam_query.p1().get_mut(*child) {
                let mut move_to_camera = if let Some(trans) = rig.move_to.1 {
                    trans
                } else {
                    *transform
                };

                // Camera Mouse Zoom
                for event in mouse_wheel_events.iter() {
                    move_to_camera.translation -=
                        move_to_camera * Vec3::ONE * event.y * rig.mouse.zoom_sensitivity;
                }

                // Camera Mouse Rotate
                if mouse_input.pressed(rig.mouse.rotate) {
                    move_to_camera.rotate(Quat::from_rotation_x(
                        -rig.mouse.rotate_sensitivity * mouse_delta_y,
                    ));
                    move_to_camera.translation =
                        Quat::from_rotation_z(-rig.mouse.rotate_sensitivity * mouse_delta_y)
                            * move_to_camera.translation;
                }

                rig.move_to.1 = Some(move_to_camera);

                // Smoothly move the camera
                if move_to_camera.translation != transform.translation {
                    if move_to_camera
                        .translation
                        .distance(transform.translation)
                        .abs()
                        > 0.005
                    {
                        transform.translation = transform.translation.lerp(
                            move_to_camera.translation,
                            time.delta().as_micros() as f32 / 100000.,
                        );
                    } else {
                        transform.translation = move_to_camera.translation;
                    }
                } else {
                    rig.move_to.0 = None;
                }
                if move_to_camera.rotation != transform.rotation {
                    if !move_to_camera
                        .rotation
                        .abs_diff_eq(transform.rotation, 0.00001)
                    {
                        transform.rotation = transform.rotation.lerp(
                            move_to_camera.rotation,
                            time.delta().as_micros() as f32 / 100000.,
                        );
                    } else {
                        transform.rotation = move_to_camera.rotation;
                    }
                } else {
                    rig.move_to.1 = None;
                }
            }
        }

        if let Ok(mut transform) = rig_cam_query.p0().get_mut(entity) {
            if *transform != rig_transform {
                *transform = rig_transform;
            }
        }
    }
}

#[derive(Component)]
pub struct CameraRigFollow(pub bool);

fn camera_rig_follow(
    time: Res<Time>,
    mut rig_query: ParamSet<(
        Query<(&mut Transform, &mut CameraRig)>,
        Query<(&Transform, &CameraRigFollow), Changed<Transform>>,
    )>,
) {
    let mut binding = rig_query.p1();
    let (follow_transform, follow) =
        if let Some((follow_transform, follow)) = binding.iter_mut().last() {
            (follow_transform.clone(), follow.clone())
        } else {
            return;
        };
    if (*follow).0 {
        for (mut transform, mut rig) in rig_query.p0().iter_mut() {
            if follow_transform.translation != transform.translation {
                if follow_transform
                    .translation
                    .distance(transform.translation)
                    .abs()
                    > 0.005
                {
                    transform.translation = transform.translation.lerp(
                        follow_transform.translation,
                        time.delta().as_micros() as f32 / 100000.,
                    );
                } else {
                    transform.translation = follow_transform.translation;
                }
            }

            // Also update the rig translation
            if let Some(mut rig_transform) = rig.move_to.0.as_mut() {
                rig_transform.translation = transform.translation;
            }
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(FourXCameraPlugin)
        .add_startup_system(setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // // camera
    // commands.spawn((
    //     Camera3dBundle {
    //         projection: OrthographicProjection {
    //             scale: 3.0,
    //             scaling_mode: ScalingMode::FixedVertical(2.0),
    //             ..default()
    //         }
    //         .into(),
    //         transform: Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    //         ..default()
    //     },
    //     Rotatable { speed: 0.3 },
    // ));

    commands
        // camera
        .spawn(CameraRigBundle::default())
        .with_children(|cb| {
            cb.spawn(Camera3dBundle {
                // I recommend setting the fov to a low value to get a
                // a pseudo-orthographic perspective
                projection: PerspectiveProjection {
                    fov: 0.1,
                    ..Default::default()
                }
                .into(),
                transform: Transform::from_translation(Vec3::new(-75.0, 75., 0.0))
                    .looking_at(Vec3::ZERO, Vec3::Y),
                ..Default::default()
            });
        });

    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(5.0).into()),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    // cubes
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(1.5, 0.5, 1.5),
        ..default()
    });
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(1.5, 0.5, -1.5),
        ..default()
    });
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(-1.5, 0.5, 1.5),
        ..default()
    });
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(-1.5, 0.5, -1.5),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(3.0, 8.0, 5.0),
        ..default()
    });
}
