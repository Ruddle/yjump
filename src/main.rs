use std::{
    io::{stdout, Write},
    time::Duration,
};

use crossterm::{
    cursor,
    event::{self, *},
    execute,
    style::{self},
    terminal,
};

use crossterm::queue;

const W: isize = 80;
const H: isize = 24;
#[derive(Clone)]
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

struct Rand(usize);

impl Rand {
    pub fn next(&mut self) -> usize {
        let mut random = self.0;
        random ^= random << 13;
        random ^= random >> 17;
        random ^= random << 5;
        self.0 = random;
        random
    }
}

struct Char {
    pos: Pos,
    old_pos: Pos,
    right_power: isize,
    jump: i32,
    fly: bool,
    down: bool,
    dy: isize,
    dx: isize,
    player: bool,
}
fn update_char(char: &mut Char, frames: usize, map: &mut MAP) {
    char.old_pos = char.pos.clone();
    if !char.fly && char.jump > 0 {
        char.dy -= 7 + if char.right_power == 0 { 1 } else { 0 };
        char.dx = char.right_power * 1;
        char.fly = true;
        char.jump = 0;
    }

    if char.fly {
        if char.down {
            char.dx = 0;
            char.dy = char.dy.max(0);
        }

        if frames % 2 == 0 {
            char.pos.x += char.dx;
        }

        char.pos.x = char.pos.x.max(1).min(W - 2);

        let mut floored = false;

        for _ in 0..if char.dy.abs() > 5 {
            1
        } else {
            if frames % 2 == 0 {
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
            char.dy = 0;
            char.dx = 0
        }
        if char.fly {
            if frames % 2 == 0 {
                char.dy += 1;
            }
        }
    }
    char.down = false;
    char.jump = (char.jump - 1).max(0);
}

fn main() -> std::io::Result<()> {
    let mut stdout = stdout();
    execute!(stdout, event::EnableFocusChange)?;
    execute!(stdout, terminal::EnterAlternateScreen)?;

    execute!(stdout, cursor::DisableBlinking)?;
    terminal::enable_raw_mode()?;

    let mut player = Char {
        pos: Pos { x: W / 2, y: H - 2 },
        old_pos: Pos { x: W / 2, y: H - 2 },
        right_power: 0_isize,
        jump: 0,
        fly: false,
        down: false,
        dy: 0_isize,
        dx: 0,
        player: true,
    };

    let mut ennemies = Vec::new();
    let rand: &mut Rand = &mut Rand(3);
    for _ in 0..2 {
        let r = (rand.next() % 30) as isize;
        let ennemy = Char {
            pos: Pos {
                x: if rand.next() % 2 == 0 { r } else { W - 1 - r },
                y: H - 2,
            },
            old_pos: Pos {
                x: 3 + W / 2,
                y: H - 2,
            },
            right_power: 0_isize,
            jump: 0,
            fly: false,
            down: false,
            dy: 0_isize,
            dx: 0,
            player: false,
        };
        ennemies.push(ennemy);
    }

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
                Event::Key(event) => {
                    match event {
                        KeyEvent {
                            code: KeyCode::Char('c'),
                            modifiers: KeyModifiers::CONTROL,
                            kind: KeyEventKind::Press,
                            state: KeyEventState::NONE,
                        }
                        | KeyEvent {
                            code: KeyCode::Esc, ..
                        } => {
                            execute!(stdout, event::DisableFocusChange)?;
                            execute!(
                                stdout,
                                style::ResetColor,
                                cursor::Show,
                                terminal::LeaveAlternateScreen
                            )?;
                            terminal::disable_raw_mode()?;
                            return Ok(());
                        }

                        KeyEvent {
                            code: KeyCode::Char('d') | KeyCode::Right,
                            ..
                        } => {
                            player.right_power = 1;
                        }
                        KeyEvent {
                            code: KeyCode::Char('q') | KeyCode::Char('a') | KeyCode::Left,
                            ..
                        } => {
                            player.right_power = -1;
                        }
                        KeyEvent {
                            code: KeyCode::Char('z') | KeyCode::Char('w') | KeyCode::Up,
                            ..
                        } => {
                            player.jump = 10;
                            // right_power = 0;
                        }
                        KeyEvent {
                            code: KeyCode::Char('s') | KeyCode::Down,
                            ..
                        } => {
                            player.down = true;
                            player.right_power = 0;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        if paused {
            std::thread::sleep(Duration::from_millis(16));
            continue;
        }

        update_char(&mut player, frames, &mut map);
        switching = (switching - 1).max(0);
        for ennemy in ennemies.iter_mut() {
            let dist = (player.pos.x - ennemy.pos.x).pow(2) + (player.pos.y - ennemy.pos.y).pow(2);

            if dist < 70 {
                ennemy.jump = 3;
                ennemy.right_power = -1 + (rand.next() % 3) as isize;
                if ennemy.right_power == 0 {
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

            update_char(ennemy, frames, &mut map);
        }
        for y in 0..H {
            for x in 0..W {
                let index = (x + y * W) as usize;
                let cell = &map[index];
                let mut color = match cell {
                    Cell::Wall => style::Color::DarkBlue,
                    Cell::Air => style::Color::Black,
                    Cell::Solid => style::Color::DarkGrey,
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
            queue!(stdout, style::SetForegroundColor(style::Color::White))?;
            queue!(stdout, style::SetBackgroundColor(style::Color::DarkBlue))?;
            queue!(stdout, cursor::MoveTo((1) as u16, 0 as u16))?;
            queue!(stdout, style::Print("yjump"))?;
            queue!(stdout, cursor::MoveTo((W / 2 - 6) as u16, 0 as u16))?;

            let alt = switching > 0 && switching % 8 < 4;

            if alt {
                queue!(stdout, style::SetAttribute(style::Attribute::Bold))?;
            }
            queue!(stdout, style::Print(format!("Score: {}", score)))?;
            if alt {
                queue!(
                    stdout,
                    style::SetAttribute(style::Attribute::NormalIntensity)
                )?;
            }
        }

        for c in ennemies.iter().chain(std::iter::once(&player)) {
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
                        char: match c.right_power {
                            1 => '>',
                            -1 => '<',
                            _ => 'Y',
                        },
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
                        style::Color::Yellow
                    } else {
                        style::Color::Green
                    },
                    front: style::Color::Black,
                    char: match c.right_power {
                        1 => '>',
                        -1 => '<',
                        _ => 'Y',
                    },
                };
            }
        }

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

        queue!(stdout, cursor::MoveTo((0) as u16, (0) as u16))?;
        queue!(stdout, style::SetBackgroundColor(style::Color::Black))?;
        queue!(stdout, style::SetForegroundColor(style::Color::DarkBlue))?;
        // queue!(stdout, style::Print(""))?;
        stdout.flush()?;
        std::thread::sleep(Duration::from_millis(16));
    }
}

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
