//! Tests of skeletons, poses and joint matrices.

use meridian_animation_domain::{
    Joint, JointId, JointMatrix, JointTransform, MAX_JOINTS, Pose, Skeleton, SkeletonError,
};
use meridian_gac::{Rotor, Vec3};

const IDENTITY: JointMatrix = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

fn translation(x: f32, y: f32, z: f32) -> JointMatrix {
    let mut m = IDENTITY;
    m[3] = [x, y, z, 1.0];
    m
}

/// `m` applied to the point `p`.
fn apply(m: JointMatrix, p: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|row| m[0][row] * p[0] + m[1][row] * p[1] + m[2][row] * p[2] + m[3][row])
}

fn assert_close(got: [f32; 3], want: [f32; 3]) {
    assert!(
        got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-5),
        "expected {want:?}, got {got:?}"
    );
}

/// A root at the origin and a child one unit up, both bound at identity.
fn arm() -> Skeleton {
    let root = Joint {
        parent: None,
        rest: JointTransform::default(),
        inverse_bind: IDENTITY,
    };
    let child = Joint {
        parent: JointId::from_index(0),
        rest: JointTransform {
            translation: Vec3::new(0.0, 1.0, 0.0),
            ..JointTransform::default()
        },
        inverse_bind: translation(0.0, -1.0, 0.0),
    };
    Skeleton::new(vec![root, child], IDENTITY).expect("a valid arm")
}

#[test]
fn the_rest_pose_leaves_a_bound_mesh_where_it_is() {
    let skeleton = arm();
    let matrices = skeleton.joint_matrices(&Pose::rest(&skeleton));
    for matrix in matrices {
        assert_close(apply(matrix, [0.3, 1.2, -0.4]), [0.3, 1.2, -0.4]);
    }
}

#[test]
fn turning_the_root_swings_the_child_with_it() {
    let skeleton = arm();
    let mut pose = Pose::rest(&skeleton);
    let turn = Rotor::from_axis_angle(Vec3::Z, core::f32::consts::FRAC_PI_2);
    pose.joint_mut(JointId::from_index(0).expect("a valid test fixture"))
        .expect("a valid test fixture")
        .rotation = turn;
    let matrices = skeleton.joint_matrices(&pose);
    // The child's vertex at (0, 1, 0) in the bind pose is a quarter turn about Z away.
    assert_close(apply(matrices[1], [0.0, 1.0, 0.0]), [-1.0, 0.0, 0.0]);
}

#[test]
fn a_childs_own_rotation_pivots_about_its_joint() {
    let skeleton = arm();
    let mut pose = Pose::rest(&skeleton);
    let turn = Rotor::from_axis_angle(Vec3::Z, core::f32::consts::FRAC_PI_2);
    pose.joint_mut(JointId::from_index(1).expect("a valid test fixture"))
        .expect("a valid test fixture")
        .rotation = turn;
    let matrices = skeleton.joint_matrices(&pose);
    // A vertex one unit above the child joint (at y = 2) swings about y = 1.
    assert_close(apply(matrices[1], [0.0, 2.0, 0.0]), [-1.0, 1.0, 0.0]);
    assert_close(apply(matrices[0], [0.0, 2.0, 0.0]), [0.0, 2.0, 0.0]);
}

#[test]
fn scale_acts_before_rotation_and_per_axis() {
    let transform = JointTransform {
        translation: Vec3::new(1.0, 0.0, 0.0),
        rotation: Rotor::from_axis_angle(Vec3::Z, core::f32::consts::FRAC_PI_2),
        scale: Vec3::new(2.0, 1.0, 1.0),
    };
    // (1, 0, 0) scales to (2, 0, 0), turns to (0, 2, 0), moves to (1, 2, 0).
    assert_close(
        apply(transform.to_matrix(), [1.0, 0.0, 0.0]),
        [1.0, 2.0, 0.0],
    );
}

#[test]
fn the_root_transform_carries_the_whole_tree() {
    let skeleton = Skeleton::new(arm().joints().to_vec(), translation(5.0, 0.0, 0.0))
        .expect("a valid test fixture");
    let matrices = skeleton.joint_matrices(&Pose::rest(&skeleton));
    assert_close(apply(matrices[1], [0.0, 1.0, 0.0]), [5.0, 1.0, 0.0]);
}

#[test]
fn a_parent_that_does_not_come_first_is_refused() {
    let mut joints = arm().joints().to_vec();
    joints.reverse();
    // The child now comes first and names joint 0, itself.
    let error = Skeleton::new(joints, IDENTITY).unwrap_err();
    assert_eq!(
        error,
        SkeletonError::ParentNotBefore {
            joint: 0,
            parent: 0
        }
    );
}

#[test]
fn too_many_joints_are_refused() {
    let joint = Joint {
        parent: None,
        rest: JointTransform::default(),
        inverse_bind: IDENTITY,
    };
    let error = Skeleton::new(vec![joint; MAX_JOINTS + 1], IDENTITY).unwrap_err();
    assert_eq!(
        error,
        SkeletonError::TooManyJoints {
            count: MAX_JOINTS + 1,
            max: MAX_JOINTS
        }
    );
}

#[test]
fn a_short_pose_keeps_the_rest_transform_of_the_joints_it_lacks() {
    let skeleton = arm();
    let matrices = skeleton.joint_matrices(&Pose::rest(&Skeleton::default()));
    assert_close(apply(matrices[1], [0.0, 1.0, 0.0]), [0.0, 1.0, 0.0]);
}

#[test]
fn blending_poses_moves_translation_scale_and_rotation_together() {
    let joint = Joint {
        parent: None,
        rest: JointTransform::default(),
        inverse_bind: IDENTITY,
    };
    let skeleton = Skeleton::new(vec![joint], IDENTITY).expect("a valid skeleton");
    let mut from = Pose::rest(&skeleton);
    let mut to = Pose::rest(&skeleton);
    to.joint_mut(JointId::from_index(0).unwrap())
        .unwrap()
        .translation = Vec3::new(4.0, 0.0, 0.0);
    to.joint_mut(JointId::from_index(0).unwrap()).unwrap().scale = Vec3::new(3.0, 1.0, 1.0);
    from.blend(&to, 0.25);
    let blended = from.transforms()[0];
    assert_close(
        [
            blended.translation.x,
            blended.translation.y,
            blended.translation.z,
        ],
        [1.0, 0.0, 0.0],
    );
    assert_close(
        [blended.scale.x, blended.scale.y, blended.scale.z],
        [1.5, 1.0, 1.0],
    );
}

#[test]
fn a_blended_rotation_takes_the_shorter_arc_halfway() {
    let quarter = Rotor::from_quaternion([
        0.0,
        0.0,
        std::f32::consts::FRAC_1_SQRT_2,
        std::f32::consts::FRAC_1_SQRT_2,
    ]);
    let a = JointTransform::default();
    let b = JointTransform {
        rotation: quarter,
        ..JointTransform::default()
    };
    let half = a.blend(b, 0.5).to_matrix();
    // Half of 90 degrees about z turns the x axis to (cos 45, sin 45, 0).
    assert_close(
        apply(half, [1.0, 0.0, 0.0]),
        [
            std::f32::consts::FRAC_1_SQRT_2,
            std::f32::consts::FRAC_1_SQRT_2,
            0.0,
        ],
    );
}
