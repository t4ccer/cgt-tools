use cgt::{misere::p_free::GameForm, short::partizan::Player};
use clap::Parser;
use quickcheck::{Arbitrary, Gen};

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long, allow_hyphen_values = true)]
    lhs: GameForm,

    #[arg(long, allow_hyphen_values = true)]
    rhs: GameForm,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let next_day = |previous_day| {
        GameForm::next_day(previous_day)
            .filter(|g: &GameForm| g.is_p_free() && g.is_dead_ending())
            .collect::<Vec<_>>()
    };

    let day0 = vec![GameForm::new_integer(0)];
    let day1 = next_day(&day0);
    let day2 = next_day(&day1);

    // for g in &day2 {
    //     eprintln!(
    //         "o({}) = {}, l = {}, r = {}",
    //         g,
    //         g.outcome(),
    //         g.tipping_point(Player::Left),
    //         g.tipping_point(Player::Right)
    //     );
    // }

    // for x in &day2 {
    //     if GameForm::sum(&args.lhs, x).outcome() != GameForm::sum(&args.rhs, x).outcome() {
    //         eprintln!("{x}");
    //     }
    // }

    // let mut rnd = Gen::new(10);
    // for _ in 0..100 {
    //     let x = GameForm::arbitrary(&mut rnd);
    //     if x.is_p_free() && x.is_dead_ending() {
    //         let ol = GameForm::sum(&args.lhs, &x).outcome();
    //         let or = GameForm::sum(&args.rhs, &x).outcome();
    //         if ol != or {
    //             eprintln!("o({} + {}) = {}", &args.lhs, x, ol);
    //             eprintln!("o({} + {}) = {}", &args.rhs, x, or);
    //             eprintln!("x = {x}");
    //             eprintln!();
    //         }
    //     }
    // }

    eprintln!("game;outcome;left tipping;right tipping");
    let mut rnd = Gen::new(8);
    for _ in 0..1000 {
        let x = GameForm::arbitrary(&mut rnd);
        if x.is_p_free() && x.is_dead_ending() {
            eprintln!(
                "{};{};{};{}",
                x,
                x.outcome(),
                x.tipping_point(Player::Left),
                x.tipping_point(Player::Right)
            );
        }
    }

    Ok(())
}
