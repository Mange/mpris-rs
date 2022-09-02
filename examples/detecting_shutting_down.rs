use std::thread::sleep;
use std::time::Duration;

use mpris::{Player, PlayerFinder};

fn move_cursor_up(n: usize) {
    print!("\x1b[{}A", n);
}

fn cursor_beginning_of_line() {
    print!("\r");
}

fn clear_to_end_of_line() {
    print!("\x1b[K");
}

fn main() {
    let finder = PlayerFinder::new().expect("Could not connect to D-Bus");
    let mut lines_drawn = 0;
    let mut all_running = false;
    let mut players: Vec<Player> = Vec::new();

    println!(
        "
All found players will be listed below, but if any of those players shuts down the
list will refresh.

TIP: Start another player that is not on the list (list will stay the same) and then shut down one
of the players on the list. You should see the running state of that player change, and then see
the list reload and your new player taking the old one's place.

Exit with Ctrl-C.
"
    );

    loop {
        if players.is_empty() {
            players = finder.find_all().expect("Could not find players");
            all_running = true;
        }

        if lines_drawn > 0 {
            cursor_beginning_of_line();
            for _ in 0..lines_drawn {
                clear_to_end_of_line();
                move_cursor_up(1);
            }
            lines_drawn = 0;
        }

        println!("Current players: ({})", players.len());
        lines_drawn += 1;
        for player in &players {
            let is_running = player.is_running();
            println!(
                "  * {} ({}) - running: {}",
                player.identity(),
                player.bus_name(),
                is_running
            );
            all_running &= is_running;
            lines_drawn += 1;
        }

        if !all_running {
            players.clear();
        }

        println!("\n(Refreshing running state every second)");
        lines_drawn += 2;
        sleep(Duration::from_secs(1));
    }
}
