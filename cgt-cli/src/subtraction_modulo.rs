use anyhow::{bail, Result};
use cgt::loopy::impartial::games::subtraction_modulo::Sub;
use clap::{self, Parser};

#[derive(Parser, Debug)]
pub struct Args {
    /// Comma separated list of values in the subtraction set
    #[arg(long, num_args=1.., value_delimiter=',')]
    moves: Vec<u32>,

    /// Starting graph size
    #[arg(long, default_value_t = 1)]
    start_n: u32,

    /// Final graph size
    #[arg(long, default_value_t = 20)]
    end_n: u32,

    #[arg(short, default_value_t = 20)]
    m: u32,
}

pub fn run(args: Args) -> Result<()> {
    if args.moves.is_empty() {
        bail!("Subtraction set cannot be empty. Use --moves a,b,... to specify it.");
    }

    // for n in args.start_n..=args.end_n {
    // 	eprintln!("{}/{}", n, args.end_n);

    //     for a in 1..args.m {
    //         for b in (a + 1)..args.m {
    //             for c in (b + 1)..args.m {
    //                 let d = b + c - a;
    //                 let moves = vec![a, b, c, d];
    //                 let sub = Sub::solve_using_graph(n, moves);
    //                 if sub
    //                     .graph
    //                     .iter()
    //                     .all(|v| matches!(v, Vertex::Value(_)))
    //                 {
    //                     println!("Graph:\n{}\n", sub);
    //                 }
    //             }
    //         }
    //     }
    // }

    // for n in args.start_n..=args.end_n {
    // 	eprintln!("{}/{}", n, args.end_n);
    // 	for a in 1..args.m {
    // 	    for d in 1..args.m {
    // 		let moves = vec![a, a+d, a+d+d, ];
    // 		let sub = Sub::solve_using_graph(n, moves);
    // 		if sub.graph.iter().any(|v| {
    // 		    matches!(v, Vertex::Value(nimber) if *nimber == Nimber(3))
    // 		}) {
    // 		    println!("Graph:\n{}\n", sub);
    // 		}
    // 	    }
    // 	}

    //     // let _sub = Sub::solve_using_sequence(&[1], n, args.moves.clone());
    //     // println!("{}", sub);
    //     // let sub = Sub::solve_using_graph(n, args.moves.clone());
    //     // println!("Graph:\n{}\n", sub);
    // }

    for n in args.start_n..=args.end_n {
        // let _sub = Sub::solve_using_sequence(&[1], n, args.moves.clone());
        // println!("{}", sub);
        let sub = Sub::solve_using_graph(n, args.moves.clone());
        println!("Graph:\n{}\n", sub);
    }

    Ok(())
}
