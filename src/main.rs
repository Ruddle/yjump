use std::{
    io::{stdout, Stdout, Write},
    time::Duration,
};

use crossterm::{
    cursor,
    event::{self, *},
    execute, queue, style, terminal,
};

const W: isize = 80;
const H: isize = 24;
const FPS: i64 = 60;
#[derive(Clone, Copy)]
struct Pos {
    x: isize,
    y: isize,
}
#[repr(u8)]
#[derive(Clone, Copy)]
enum Cell {
    Air,
    Solid,
    Wall,
}
#[derive(Clone, Copy)]
struct Pixel {
    back: style::Color,
    front: style::Color,
    char: char,
}

type MAP = [Cell; (W * H) as usize];
fn gen_map(rand: &mut Rand) -> MAP {
    let mut map = [Cell::Air; (W * H) as usize];
    for x in 0..W {
        for y in 0..H {
            let index = (x + y * W) as usize;
            let border = x == 0 || x == W - 1 || y == 0 || y == H - 1;
            if border {
                map[index] = Cell::Wall;
            } else {
                let r = rand.next() % 60;

                if r < 1 {
                    map[index] = Cell::Solid;
                    let len = 2 + (rand.next() % 2) as isize;
                    for u in -len..len {
                        if x + u > 0 && x + u < W - 1 {
                            let index = (x + u + y * W) as usize;
                            map[index] = Cell::Solid;
                        }
                    }
                }
            }
        }
    }
    map
}

struct Rand(usize);

impl Rand {
    pub fn next(&mut self) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Dash {
    Ready,
    Dashing(usize),
    Loading(usize),
}
struct Char {
    pos: Pos,
    old_pos: Pos,
    right_power: isize,
    last_power_frame: isize,
    jump: i32,
    double_jump_ready: bool,
    fly: bool,
    down: bool,
    dy: isize,
    dx: isize,
    player: bool,
    dash: Dash,
    phase: isize,
}
fn update_char(
    char: &mut Char,
    frames: isize,
    map: &mut MAP,
    rand: &mut Rand,
    particles: &mut Vec<Particle>,
) {
    char.dash = match char.dash {
        Dash::Loading(x) if x > 0 => Dash::Loading(x - 1),
        Dash::Dashing(x) if x > 0 => Dash::Dashing(x - 1),
        Dash::Dashing(_) => Dash::Loading(60),
        _ => Dash::Ready,
    };
    char.old_pos = char.pos.clone();
    let dashing = if let Dash::Dashing(_) = char.dash {
        true
    } else {
        false
    };

    let color = if char.player {
        style::Color::Yellow
    } else {
        style::Color::Green
    };

    if dashing {
        char.pos.x += char.right_power;
        char.pos.x = char.pos.x.max(1).min(W - 2);
        char.dx = char.right_power;
        char.dy = 0;
        return;
    }

    let fly0 = char.fly;
    if !char.fly && char.jump > 0 {
        char.double_jump_ready = true;
        char.dy -= 5 + if char.right_power == 0 { 1 } else { 0 };
        char.dx = char.right_power * 1;
        char.fly = true;
        char.jump = 0;
    }

    if !char.fly {
        let cell = map[(char.pos.x + (char.pos.y + 1) * W) as usize];
        match cell {
            Cell::Solid | Cell::Wall => {}
            _ => {
                char.fly = true;
            }
        }
    }

    if char.fly && !fly0 {
        char.phase = frames;

        spawn_particles(particles, char.pos, rand, color);
    }

    let alt3: bool = (frames - char.phase + 1) % 3 == 0;

    if char.fly {
        if char.jump > 0 && char.double_jump_ready {
            char.double_jump_ready = false;
            char.dy = 0;
            char.dy -= 5 + if char.right_power == 0 { 1 } else { 0 };
            char.dx = char.right_power * 1;
        }

        if char.down {
            char.dx = 0;
            char.dy = char.dy.max(0);
        }

        if alt3 {
            char.pos.x += char.dx;
        }

        char.pos.x = char.pos.x.max(1).min(W - 2);

        let mut floored = false;

        for _ in 0..if char.dy.abs() >= 5 {
            1
        } else {
            if alt3 {
                1
            } else {
                0
            }
        } {
            let mut ceiled = false;
            if char.dy.signum() > 0 {
                let cell = map[(char.pos.x + (char.pos.y + 1) * W) as usize];
                match cell {
                    Cell::Solid | Cell::Wall => {
                        floored = true;
                    }
                    _ => {}
                }
            }
            if char.dy.signum() < 0 {
                let cell = map[(char.pos.x + (char.pos.y - 1) * W) as usize];
                match cell {
                    Cell::Wall => ceiled = true,
                    _ => {}
                }
            }

            if ceiled {
                char.dy = 0
            }

            if floored {
                char.dy = 0;
            }
            if !ceiled && !floored {
                char.pos.y += char.dy.signum();
            }
        }

        if floored {
            char.fly = false;
            spawn_particles(particles, char.pos, rand, color);
            char.dy = 0;
            char.dx = 0
        }
        if char.fly {
            if alt3 {
                char.dy += 1;
            }
        }
    }
    char.down = false;
    char.jump = (char.jump - 1).max(0);
}

struct Particle {
    p: Pos,
    dx: isize,
    dy: isize,
    life: isize,
    kind: char,
    color: style::Color,
}

fn spawn_particles(particles: &mut Vec<Particle>, pos: Pos, rand: &mut Rand, color: style::Color) {
    for _ in 0..5 {
        let p = Particle {
            p: pos,
            dx: -3 + (rand.next() % 7) as isize,
            dy: -3 + (rand.next() % 5) as isize,
            life: 10 + (rand.next() % 10) as isize,
            kind: ['*', '.', '¨', '¤', '\'', '²'][rand.next() % 6],
            color,
        };
        particles.push(p);
    }
}
fn main() -> std::io::Result<()> {
    let mut stdout = stdout();
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        event::EnableFocusChange,
        cursor::DisableBlinking,
        cursor::Hide
    )?;
    terminal::enable_raw_mode()?;

    let _ = game(&mut stdout);

    execute!(
        stdout,
        cursor::Show,
        style::ResetColor,
        event::DisableFocusChange,
        terminal::LeaveAlternateScreen
    )?;
    terminal::disable_raw_mode()?;
    Ok(())
}

fn game(stdout: &mut Stdout) -> std::io::Result<()> {
    let rand = &mut Rand(5);

    let mut player = Char {
        pos: Pos { x: W / 2, y: H - 2 },
        old_pos: Pos { x: W / 2, y: H - 2 },
        right_power: 0_isize,
        last_power_frame: 0,
        jump: 0,
        double_jump_ready: true,
        fly: false,
        down: false,
        dy: 0_isize,
        dx: 0,
        player: true,
        dash: Dash::Ready,
        phase: 0,
    };

    let mut enemies = Vec::new();
    for _ in 0..2 {
        let r = (rand.next() % 30) as isize;
        let char = Char {
            pos: Pos {
                x: if rand.next() % 2 == 0 { r } else { W - 1 - r },
                y: H - 2,
            },
            old_pos: Pos {
                x: 3 + W / 2,
                y: H - 2,
            },
            right_power: 0_isize,
            last_power_frame: 0,
            jump: 0,
            double_jump_ready: true,
            fly: false,
            down: false,
            dy: 0_isize,
            dx: 0,
            player: false,
            dash: Dash::Ready,
            phase: 0,
        };
        enemies.push(char);
    }

    let mut particles: Vec<Particle> = Vec::new();

    let mut paused = false;

    let mut pixels = [Pixel {
        back: style::Color::Black,
        front: style::Color::Black,
        char: ' ',
    }; (W * H) as usize];

    let mut pixels_drawn = [Pixel {
        back: style::Color::Black,
        front: style::Color::Black,
        char: '£',
    }; (W * H) as usize];

    let mut map = gen_map(rand);
    let mut frames = 0;
    let mut score = 0;
    let mut switching = 0;

    loop {
        let start = std::time::Instant::now();
        frames += 1;
        while poll(Duration::from_millis(0))? {
            let e = read()?;
            match e {
                Event::FocusGained if paused => {
                    std::thread::sleep(Duration::from_millis(200));
                    pixels_drawn = [Pixel {
                        back: style::Color::Black,
                        front: style::Color::Black,
                        char: '£',
                    }; (W * H) as usize];

                    paused = false
                }
                Event::FocusLost if !paused => {
                    queue!(
                        stdout,
                        cursor::MoveTo((W / 2 - 3) as u16, (H / 2 - 1) as u16)
                    )?;
                    queue!(stdout, style::SetBackgroundColor(style::Color::Red))?;
                    queue!(stdout, style::SetForegroundColor(style::Color::Black))?;
                    queue!(stdout, style::SetAttribute(style::Attribute::Bold))?;
                    queue!(stdout, style::Print("PAUSED"))?;
                    queue!(
                        stdout,
                        style::SetAttribute(style::Attribute::NormalIntensity)
                    )?;
                    queue!(stdout, cursor::MoveTo((W - 1) as u16, (H - 1) as u16))?;
                    stdout.flush()?;
                    paused = true
                }
                Event::Key(event) => match event {
                    KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    }
                    | KeyEvent {
                        code: KeyCode::Esc, ..
                    } => {
                        return Ok(());
                    }

                    KeyEvent {
                        code: KeyCode::Char('d') | KeyCode::Right,
                        ..
                    } => {
                        if player.dash == Dash::Ready
                            && player.right_power == 1
                            && (frames - player.last_power_frame) < 20
                        {
                            player.dash = Dash::Dashing(20)
                        }
                        if let Dash::Dashing(_) = player.dash {
                        } else {
                            player.right_power = 1;
                            player.last_power_frame = frames;
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Char('q') | KeyCode::Char('a') | KeyCode::Left,
                        ..
                    } => {
                        if player.dash == Dash::Ready
                            && player.right_power == -1
                            && (frames - player.last_power_frame) < 20
                        {
                            player.dash = Dash::Dashing(20)
                        }
                        if let Dash::Dashing(_) = player.dash {
                        } else {
                            player.right_power = -1;
                            player.last_power_frame = frames;
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Char('z') | KeyCode::Char('w') | KeyCode::Up,
                        ..
                    } => {
                        player.jump = 10;
                    }
                    KeyEvent {
                        code: KeyCode::Char('s') | KeyCode::Down,
                        ..
                    } => {
                        player.down = true;
                        player.right_power = 0;
                    }
                    _ => {}
                },
                Event::Resize(_, _) => {
                    pixels_drawn = [Pixel {
                        back: style::Color::Black,
                        front: style::Color::Black,
                        char: '£',
                    }; (W * H) as usize];
                }
                _ => {}
            }
        }
        if paused {
            std::thread::sleep(Duration::from_millis(16));
            continue;
        }

        update_char(&mut player, frames, &mut map, rand, &mut particles);
        switching = (switching - 1).max(0);
        for ennemy in enemies.iter_mut() {
            let dist = (player.pos.x - ennemy.pos.x).pow(2) + (player.pos.y - ennemy.pos.y).pow(2);

            if dist < 70 {
                ennemy.jump = 3;
                ennemy.right_power = -1 + (rand.next() % 3) as isize;
                if ennemy.right_power == 0
                    || (ennemy.pos.x == 1 && ennemy.right_power == -1)
                    || (ennemy.pos.x == W - 2 && ennemy.right_power == 1)
                {
                    ennemy.right_power = -1 + (rand.next() % 3) as isize;
                }
            } else {
                ennemy.jump = 0
            }

            if switching == 0 && player.pos.x == ennemy.pos.x && player.pos.y == ennemy.pos.y {
                switching = 60;
                map = gen_map(rand);
                score += 1;
            }

            update_char(ennemy, frames, &mut map, rand, &mut particles);
        }

        for y in 0..H {
            for x in 0..W {
                let index = (x + y * W) as usize;
                let cell = &map[index];
                let mut color = match cell {
                    Cell::Wall => style::Color::DarkBlue,
                    Cell::Air => style::Color::Black,
                    Cell::Solid => style::Color::DarkBlue,
                };
                if switching > 0 {
                    color = style::Color::DarkBlue;
                }
                pixels[index] = Pixel {
                    back: color,
                    front: color,
                    char: ' ',
                };
            }
        }

        {
            let mut x = 1;
            for c in "yjump ".chars().chain(env!("CARGO_PKG_VERSION").chars()) {
                pixels[x] = Pixel {
                    back: style::Color::DarkBlue,
                    front: style::Color::White,
                    char: c,
                };
                x += 1;
            }
        }
        {
            let alt = switching > 0 && switching % 8 < 4;
            let sep = if alt { '-' } else { ' ' };

            let s = format!("{}Score: {}{}", sep, score, sep);
            let mut x = W as usize / 2 - s.len() / 2;
            for c in s.chars() {
                pixels[x] = Pixel {
                    back: style::Color::DarkBlue,
                    front: style::Color::White,
                    char: c,
                };
                x += 1;
            }
        }

        for c in enemies.iter().chain(std::iter::once(&player)) {
            let logo = match c.right_power {
                1 => '>',
                -1 => '<',
                _ => 'Y',
            };
            {
                let mut sx = c.old_pos.x;
                let mut sy = c.old_pos.y;
                while sx != c.pos.x || sy != c.pos.y {
                    let index = (sx + sy * W) as usize;
                    pixels[index] = Pixel {
                        back: if c.player {
                            style::Color::DarkYellow
                        } else {
                            style::Color::DarkGreen
                        },
                        front: style::Color::Black,
                        char: logo,
                    };
                    let dy = (c.pos.y - sy).signum();
                    sy += dy;
                    let dx = (c.pos.x - sx).signum();
                    sx += dx;
                }
            }
            {
                let index = (c.pos.x + c.pos.y * W) as usize;
                pixels[index] = Pixel {
                    back: if c.player {
                        match c.dash {
                            Dash::Dashing(_) => style::Color::White,
                            Dash::Loading(_) => style::Color::DarkYellow,
                            Dash::Ready => style::Color::Yellow,
                        }
                    } else {
                        style::Color::Green
                    },
                    front: style::Color::Black,
                    char: logo,
                };
            }
        }

        for p in particles.iter_mut() {
            p.life -= 1;

            if frames % (6 - p.dx.abs()) == 0 {
                p.p.x += p.dx.signum();
            }
            if frames % (6 - p.dy.abs()).max(1) == 0 {
                p.p.y += p.dy.signum();
            }

            if p.life % 4 == 0 && p.dy < 3 {
                p.dy += 1;
            }

            if p.p.x < 1 || p.p.x >= W - 1 || p.p.y < 1 || p.p.y >= H - 1 {
                continue;
            }
            let index = (p.p.x + p.p.y * W) as usize;
            if let Pixel {
                back: style::Color::Black,
                front: style::Color::Black,
                char: ' ',
            } = pixels[index]
            {
                pixels[index] = Pixel {
                    back: style::Color::Black,
                    front: p.color,
                    char: p.kind,
                }
            }
        }

        particles.retain(|p| p.life > 0);

        // Queue dirty pixels
        for (index, (p, pd)) in pixels.iter().zip(pixels_drawn.iter_mut()).enumerate() {
            if p.back != pd.back || p.front != pd.front || p.char != pd.char {
                *pd = *p;
                queue!(
                    stdout,
                    cursor::MoveTo((index % W as usize) as u16, (index / W as usize) as u16)
                )?;
                queue!(stdout, style::SetForegroundColor(p.front))?;
                queue!(stdout, style::SetBackgroundColor(p.back))?;
                queue!(stdout, style::Print(p.char))?;
            }
        }
        stdout.flush()?;
        let diff = 1_000_000 / FPS - start.elapsed().as_micros() as i64;
        if diff > 0 {
            std::thread::sleep(Duration::from_micros(diff as u64));
        }
    }
}
