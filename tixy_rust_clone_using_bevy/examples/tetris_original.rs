// https://github.com/maxicarv/follow_bevy_tetris

use bevy::prelude::*;
use rand::prelude::*;

const UNIT_WIDTH: u32 = 40;
const UNIT_HEIGHT: u32 = 40;

const X_LENGTH: u32 = 10;
const Y_LENGTH: u32 = 18;

const SCREEN_WIDTH: u32 = UNIT_WIDTH * X_LENGTH;
const SCREEN_HEIGHT: u32 = UNIT_HEIGHT * Y_LENGTH;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct RelativePosition {
    x: i32,
    y: i32,
}
#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}
#[derive(Component, Clone, PartialEq, Eq)]
struct BlockPatterns(Vec<Vec<(i32, i32)>>);

#[derive(Component)]
struct Fix;
#[derive(Component)]
struct Free;

struct NewBlockEvent;
struct GameOverEvent;
struct GameTimer(Timer);
struct InputTimer(Timer);
struct GameBoard(Vec<Vec<bool>>);

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Tetris".to_string(),
            width: SCREEN_WIDTH as f32,
            height: SCREEN_HEIGHT as f32,
            ..Default::default()
        })
        .insert_resource(BlockPatterns(vec![
            vec![(0, 0), (0, -1), (0, 1), (0, 2)],  // I
            vec![(0, 0), (0, -1), (0, 1), (-1, 1)], // L
            vec![(0, 0), (0, -1), (0, 1), (1, 1)],  // 逆L
            vec![(0, 0), (0, -1), (1, 0), (1, 1)],  // Z
            vec![(0, 0), (1, 0), (0, 1), (1, -1)],  // 逆Z
            vec![(0, 0), (0, 1), (1, 0), (1, 1)],   // 四角
            vec![(0, 0), (-1, 0), (1, 0), (0, 1)],  // T
        ]))
        .insert_resource(GameTimer(Timer::new(
            std::time::Duration::from_millis(400),
            true,
        )))
        .insert_resource(InputTimer(Timer::new(
            std::time::Duration::from_millis(400),
            true,
        )))
        .insert_resource(GameBoard(vec![vec![false; 25]; 25]))
        .add_event::<NewBlockEvent>()
        .add_event::<GameOverEvent>()
        .add_startup_system(setup_camera)
        .add_startup_system(setup)
        .add_system(gameover)
        .add_system(delete_line)
        .add_system(position_transform)
        .add_system(spawn_block)
        .add_system(game_timer)
        .add_system(block_fall)
        .add_system(block_horizontal_move)
        .add_system(block_vertical_move)
        .add_system(block_rotate)
        .add_plugins(DefaultPlugins)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

//See to URL 'https://bevyengine.org/learn/book/migration-guides/0.4-0.5/#simplified-events'
fn setup(mut new_block_events: EventWriter<NewBlockEvent>) {
    new_block_events.send(NewBlockEvent);
}

fn position_transform(mut position_query: Query<(&Position, &mut Transform, &mut Sprite)>) {
    let origin_x = UNIT_WIDTH as i32 / 2 - SCREEN_WIDTH as i32 / 2;
    let origin_y = UNIT_HEIGHT as i32 / 2 - SCREEN_HEIGHT as i32 / 2;
    position_query
        .iter_mut()
        .for_each(|(pos, mut transform, mut sprite)| {
            transform.translation = Vec3::new(
                (origin_x + pos.x as i32 * UNIT_WIDTH as i32) as f32,
                (origin_y + pos.y as i32 * UNIT_HEIGHT as i32) as f32,
                0.0,
            );
            sprite.custom_size = Some(Vec2::new(UNIT_WIDTH as f32, UNIT_HEIGHT as f32))
        });
}

fn spawn_block_element(
    commands: &mut Commands,
    color: Color,
    position: Position,
    relative_position: RelativePosition,
) {
    //See to URL 'https://bevyengine.org/learn/book/migration-guides/0.5-0.6/#spritebundle-and-sprite'
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(position)
        .insert(relative_position)
        .insert(Free);
}

fn next_color() -> Color {
    let colors = vec![
        Color::rgb_u8(64, 230, 100),
        Color::rgb_u8(220, 64, 90),
        Color::rgb_u8(70, 150, 210),
        Color::rgb_u8(220, 230, 70),
        Color::rgb_u8(35, 220, 241),
        Color::rgb_u8(240, 140, 70),
    ];
    let mut rng = rand::thread_rng();
    let mut color_index: usize = rng.gen();
    color_index %= colors.len();
    colors[color_index].clone()
}

fn next_block(block_patterns: &Vec<Vec<(i32, i32)>>) -> Vec<(i32, i32)> {
    let mut rng = rand::thread_rng();
    let mut pattern_index: usize = rng.gen();
    pattern_index %= block_patterns.len();

    block_patterns[pattern_index].clone()
}

fn spawn_block(
    mut commands: Commands,
    block_patterns: Res<BlockPatterns>,
    //See to URL 'https://bevyengine.org/learn/book/migration-guides/0.4-0.5/#simplified-events'
    mut new_block_events_reader: EventReader<NewBlockEvent>,
    game_board: ResMut<GameBoard>,
    mut gameover_event: EventWriter<GameOverEvent>,
) {
    if new_block_events_reader.iter().next().is_none() {
        return;
    }

    let new_block = next_block(&block_patterns.0);
    let new_color = next_color();

    // ブロックの初期位置
    let initial_x = X_LENGTH / 2;
    let initial_y = Y_LENGTH;

    // ゲームオーバー判定
    let gameover = new_block.iter().any(|(r_x, r_y)| {
        let pos_x = (initial_x as i32 + r_x) as usize;
        let pos_y = (initial_y as i32 + r_y) as usize;

        game_board.0[pos_y][pos_x]
    });

    if gameover {
        gameover_event.send(GameOverEvent);
        return;
    }

    new_block.iter().for_each(|(r_x, r_y)| {
        spawn_block_element(
            &mut commands,
            new_color.clone(),
            Position {
                x: (initial_x as i32 + r_x),
                y: (initial_y as i32 + r_y),
            },
            RelativePosition { x: *r_x, y: *r_y },
        );
    });
}

fn game_timer(
    time: Res<Time>,
    mut game_timer: ResMut<GameTimer>,
    mut imput_timer: ResMut<InputTimer>,
) {
    //See to URL 'https://bevyengine.org/learn/book/migration-guides/0.4-0.5/#timer-now-uses-duration'
    game_timer.0.tick(time.delta());
    imput_timer.0.tick(time.delta());
}

fn block_fall(
    mut commands: Commands,
    timer: ResMut<GameTimer>,
    mut block_query: Query<(Entity, &mut Position, &Free)>,
    mut game_board: ResMut<GameBoard>,
    mut new_block_events: EventWriter<NewBlockEvent>,
) {
    if !timer.0.finished() {
        return;
    }

    let cannot_fall = block_query.iter_mut().any(|(_, pos, _)| {
        if pos.x as u32 >= X_LENGTH || pos.y as u32 >= Y_LENGTH {
            return false;
        }
        // yが0、または一つ下にブロックがすでに存在する
        pos.y == 0 || game_board.0[(pos.y - 1) as usize][pos.x as usize]
    });

    if cannot_fall {
        block_query.iter_mut().for_each(|(entity, pos, _)| {
            //See to URL 'https://bevyengine.org/learn/book/migration-guides/0.4-0.5/#commands-api'
            commands.entity(entity).remove::<Free>();
            commands.entity(entity).insert(Fix);
            game_board.0[pos.y as usize][pos.x as usize] = true;
        });
        new_block_events.send(NewBlockEvent);
    } else {
        block_query.iter_mut().for_each(|(_, mut pos, _)| {
            pos.y -= 1;
        });
    }
}

fn block_horizontal_move(
    key_input: Res<Input<KeyCode>>,
    timer: ResMut<InputTimer>,
    game_board: ResMut<GameBoard>,
    mut free_block_query: Query<(Entity, &mut Position, &Free)>,
) {
    if !timer.0.finished() {
        return;
    }
    if key_input.pressed(KeyCode::Left) {
        // 左に移動できるか判定
        let ok_move_left = free_block_query.iter_mut().all(|(_, pos, _)| {
            if pos.y as u32 >= Y_LENGTH {
                return pos.x > 0;
            }

            if pos.x == 0 {
                return false;
            }

            !game_board.0[(pos.y) as usize][pos.x as usize - 1]
        });

        if ok_move_left {
            free_block_query.iter_mut().for_each(|(_, mut pos, _)| {
                pos.x -= 1;
            });
        }
    }

    if key_input.pressed(KeyCode::Right) {
        // 右に移動できるか判定
        let ok_move_right = free_block_query.iter_mut().all(|(_, pos, _)| {
            if pos.y as u32 >= Y_LENGTH {
                return pos.x as u32 <= X_LENGTH;
            }

            if pos.x as u32 == X_LENGTH - 1 {
                return false;
            }

            !game_board.0[(pos.y) as usize][pos.x as usize + 1]
        });

        if ok_move_right {
            free_block_query.iter_mut().for_each(|(_, mut pos, _)| {
                pos.x += 1;
            });
        }
    }
}

fn block_vertical_move(
    key_input: Res<Input<KeyCode>>,
    mut game_board: ResMut<GameBoard>,
    mut free_block_query: Query<(Entity, &mut Position, &Free)>,
) {
    if !key_input.just_pressed(KeyCode::Down) {
        return;
    }

    let mut down_height = 0;
    let mut collide = false;

    // ブロックが衝突する位置を調べる
    while !collide {
        down_height += 1;
        free_block_query.iter_mut().for_each(|(_, pos, _)| {
            if pos.y < down_height {
                collide = true;
                return;
            }

            if game_board.0[(pos.y - down_height) as usize][pos.x as usize] {
                collide = true;
            }
        });
    }

    // ブロックが衝突しないギリギリの位置まで移動
    down_height -= 1;
    free_block_query.iter_mut().for_each(|(_, mut pos, _)| {
        game_board.0[pos.y as usize][pos.x as usize] = false;
        pos.y -= down_height;
        game_board.0[pos.y as usize][pos.x as usize] = true;
    });
}

fn block_rotate(
    key_input: Res<Input<KeyCode>>,
    game_board: ResMut<GameBoard>,
    mut free_block_query: Query<(Entity, &mut Position, &mut RelativePosition, &Free)>,
) {
    if !key_input.just_pressed(KeyCode::Up) {
        return;
    }

    // 回転行列を使って新しい絶対座標と相対座標を計算
    fn calc_rotated_pos(pos: &Position, r_pos: &RelativePosition) -> ((i32, i32), (i32, i32)) {
        // cos,-sin,sin,cos (-90)
        let rot_matrix = vec![vec![0, 1], vec![-1, 0]];

        let origin_pos_x = pos.x - r_pos.x;
        let origin_pos_y = pos.y - r_pos.y;

        let new_r_pos_x = rot_matrix[0][0] * r_pos.x + rot_matrix[0][1] * r_pos.y;
        let new_r_pos_y = rot_matrix[1][0] * r_pos.x + rot_matrix[1][1] * r_pos.y;
        let new_pos_x = origin_pos_x + new_r_pos_x;
        let new_pos_y = origin_pos_y + new_r_pos_y;

        ((new_pos_x, new_pos_y), (new_r_pos_x, new_r_pos_y))
    }

    // 回転操作可能かどうか判定
    let rotable = free_block_query.iter_mut().all(|(_, pos, r_pos, _)| {
        let ((new_pos_x, new_pos_y), _) = calc_rotated_pos(&pos, &r_pos);

        let valid_index_x = new_pos_x >= 0 && new_pos_x < X_LENGTH as i32;
        let valid_index_y = new_pos_y >= 0 && new_pos_y < Y_LENGTH as i32;

        if !valid_index_x || !valid_index_y {
            return false;
        }

        !game_board.0[new_pos_y as usize][new_pos_x as usize]
    });

    if !rotable {
        return;
    }

    // 相対座標と絶対座標を更新
    free_block_query
        .iter_mut()
        .for_each(|(_, mut pos, mut r_pos, _)| {
            let ((new_pos_x, new_pos_y), (new_r_pos_x, new_r_pos_y)) =
                calc_rotated_pos(&pos, &r_pos);
            r_pos.x = new_r_pos_x;
            r_pos.y = new_r_pos_y;

            pos.x = new_pos_x;
            pos.y = new_pos_y;
        });
}

fn delete_line(
    mut commands: Commands,
    timer: ResMut<GameTimer>,
    mut game_board: ResMut<GameBoard>,
    mut fixed_block_query: Query<(Entity, &mut Position, &Fix)>,
) {
    if !timer.0.finished() {
        return;
    }

    // 消去対象のブロック行をHashSetに入れていく
    let mut delete_line_set = std::collections::HashSet::new();
    for y in 0..Y_LENGTH {
        let mut delete_current_line = true;
        for x in 0..X_LENGTH {
            if !game_board.0[y as usize][x as usize] {
                delete_current_line = false;
                break;
            }
        }

        if delete_current_line {
            delete_line_set.insert(y);
        }
    }

    // 消去対象ブロック行に含まれるブロックをゲーム盤面から削除する
    fixed_block_query.iter_mut().for_each(|(_, pos, _)| {
        if delete_line_set.get(&(pos.y as u32)).is_some() {
            game_board.0[pos.y as usize][pos.x as usize] = false;
        }
    });

    // 各Y座標について、ブロック消去適用後の新しいY座標を調べる
    let mut new_y = vec![0i32; Y_LENGTH as usize + 10];
    for y in 0..Y_LENGTH {
        let mut down = 0;
        delete_line_set.iter().for_each(|line| {
            if y > *line {
                down += 1;
            }
        });
        new_y[y as usize] = y as i32 - down;
    }

    fixed_block_query
        .iter_mut()
        .for_each(|(entity, mut pos, _)| {
            if delete_line_set.get(&(pos.y as u32)).is_some() {
                // 消去の対象のブロックをゲームから取り除く
                commands.entity(entity).despawn();
            } else {
                // ブロック消去適用後の新しいY座標を適用
                game_board.0[pos.y as usize][pos.x as usize] = false;
                pos.y = new_y[pos.y as usize];
                game_board.0[pos.y as usize][pos.x as usize] = true;
            }
        });
}

fn gameover(
    mut commands: Commands,
    //See to URL 'https://bevyengine.org/learn/book/migration-guides/0.4-0.5/#simplified-events'
    mut gameover_events: EventReader<GameOverEvent>,
    mut game_board: ResMut<GameBoard>,
    mut all_block_query: Query<(Entity, &mut Position)>,
    mut new_block_events: EventWriter<NewBlockEvent>,
) {
    if gameover_events.iter().next().is_none() {
        return;
    }

    game_board.0 = vec![vec![false; 25]; 25];
    all_block_query.iter_mut().for_each(|(ent, _)| {
        commands.entity(ent).despawn();
    });

    new_block_events.send(NewBlockEvent);
}
