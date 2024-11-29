mod types;
use types::{Stock, Order, Trade};
// use tokio::sync::mpsc;


use std::io;

fn main() {
    println!("D1X Demo - running with one stock");
    let mut v = Stock::new("V", "Vulyenne");

    let mut input = String::new();
    println!("{}({}): {:#?}", v.get_name(), v.get_symbol(), v.get_ohlc());

    loop {
        println!("Enter a command: ");
        input.clear();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        match input {
            "buy" => {
                println!("Enter creator id: ");
                let mut creator_id = String::new();
                io::stdin().read_line(&mut creator_id).unwrap();
                let creator_id: usize = creator_id.trim().parse().unwrap();

                println!("Enter price: ");
                let mut price = String::new();
                io::stdin().read_line(&mut price).unwrap();
                let price: f64 = price.trim().parse().unwrap();

                println!("Enter quantity: ");
                let mut quantity = String::new();
                io::stdin().read_line(&mut quantity).unwrap();
                let quantity: usize = quantity.trim().parse().unwrap();

                v.add_buy_order(Order::new(creator_id, price, quantity));
                println!("Buy order added.");
            }
            "sell" => {
                println!("Enter creator id: ");
                let mut creator_id = String::new();
                io::stdin().read_line(&mut creator_id).unwrap();
                let creator_id: usize = creator_id.trim().parse().unwrap();

                println!("Enter price: ");
                let mut price = String::new();
                io::stdin().read_line(&mut price).unwrap();
                let price: f64 = price.trim().parse().unwrap();

                println!("Enter quantity: ");
                let mut quantity = String::new();
                io::stdin().read_line(&mut quantity).unwrap();
                let quantity: usize = quantity.trim().parse().unwrap();

                v.add_sell_order(Order::new(creator_id, price, quantity));
                println!("Sell order added.");
            }
            "resolve" => {
                let trades = v.resolve();
                println!("Trades: {:#?}", trades);
            }
            "exit" => {
                break;
            }
            _ => {
                println!("Invalid command");
            }
        }
    }
}
