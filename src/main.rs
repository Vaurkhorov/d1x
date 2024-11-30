mod types;

use std::collections::HashMap;
use std::io;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use tokio::sync::mpsc;
use tokio::task;
use tokio::time;
use types::{Order, Query, QueryResponse, Stock};

const TICK_INTERVAL_MILLISECS: u64 = 10;
const MARKET_OUTPUT_COLOUR: Color = Color::Yellow;

#[tokio::main]
async fn main() {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    let (server_tx, mut market_rx) = mpsc::channel::<Query>(32);
    let (market_tx, mut server_rx) = mpsc::channel::<QueryResponse>(32);

    let mut stocks: HashMap<String, Stock> = HashMap::new();
    let mut initial_stocks = vec![Stock::new("V", "Vulyenne")];

    while let Some(stock) = initial_stocks.pop() {
        stocks.insert(stock.get_symbol().to_string(), stock);
    }

    let server = task::spawn(async move {
        let mut input = String::new();

        loop {
            println!("Enter a command: ");
            input.clear();
            io::stdin().read_line(&mut input).unwrap();
            let input = input.trim();

            let status = match input {
                "buy" => {
                    println!("Enter stock symbol: ");
                    let mut symbol = String::new();
                    io::stdin().read_line(&mut symbol).unwrap();
                    let symbol: String = symbol.trim().parse().unwrap();

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

                    let mpsc_status = server_tx
                        .send(Query::Buy(symbol, Order::new(creator_id, price, quantity)))
                        .await;
                    println!("Buy order added.");
                    mpsc_status
                }
                "sell" => {
                    println!("Enter stock symbol: ");
                    let mut symbol = String::new();
                    io::stdin().read_line(&mut symbol).unwrap();
                    let symbol: String = symbol.trim().parse().unwrap();

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

                    let mpsc_status = server_tx
                        .send(Query::Sell(symbol, Order::new(creator_id, price, quantity)))
                        .await;
                    println!("Sell order added.");
                    mpsc_status
                }
                "exit" => {
                    break;
                }
                _ => {
                    println!("Invalid command\n");
                    continue;
                }
            };

            if let Err(e) = status {
                eprintln!("Error: {:#?}", e);
                continue;
            }

            match server_rx.recv().await {
                Some(response) => match response {
                    QueryResponse::OrderPosted => {
                        println!("Order posted.");
                    }
                    QueryResponse::SymbolNotFound => {
                        println!("Symbol not found.");
                    }
                    QueryResponse::Ohlc(open, high, low, close) => {
                        println!("OHLC: {:#?}", (open, high, low, close));
                    }
                    QueryResponse::QueriedOrders(orders) => {
                        println!("Orders: {:#?}", orders);
                    }
                },
                None => {
                    break;
                }
            }
        }
    });

    let mut tick_interval = time::interval(time::Duration::from_millis(TICK_INTERVAL_MILLISECS));
    tick_interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    'market_loop: loop {
        tick_interval.tick().await;
        loop {
            for stock in stocks.values_mut() {
                let executed_trades = stock.resolve();
                for trade in executed_trades {
                    market_speak(
                        format!("Market says> Trade executed: {:#?}", trade),
                        &mut stdout,
                        false,
                    );
                }
            }

            match market_rx.try_recv() {
                Ok(query) => {
                    let status = match query {
                        Query::Buy(symbol, order) => {
                            if let Some(stock) = stocks.get_mut(&symbol) {
                                stock.add_buy_order(order);
                                market_tx.send(QueryResponse::OrderPosted).await
                            } else {
                                market_tx.send(QueryResponse::SymbolNotFound).await
                            }
                        }
                        Query::Sell(symbol, order) => {
                            if let Some(stock) = stocks.get_mut(&symbol) {
                                stock.add_sell_order(order);
                                market_tx.send(QueryResponse::OrderPosted).await
                            } else {
                                market_tx.send(QueryResponse::SymbolNotFound).await
                            }
                        }
                        Query::Ohlc(symbol) => {
                            if let Some(stock) = stocks.get(&symbol) {
                                let (open, high, low, close) = stock.get_ohlc();
                                market_tx
                                    .send(QueryResponse::Ohlc(open, high, low, close))
                                    .await
                            } else {
                                market_tx.send(QueryResponse::SymbolNotFound).await
                            }
                        }
                        Query::BuyOrders(symbol) => {
                            if let Some(stock) = stocks.get(&symbol) {
                                market_tx
                                    .send(QueryResponse::QueriedOrders(stock.get_buy_orders()))
                                    .await
                            } else {
                                market_tx.send(QueryResponse::SymbolNotFound).await
                            }
                        }
                        Query::SellOrders(symbol) => {
                            if let Some(stock) = stocks.get(&symbol) {
                                market_tx
                                    .send(QueryResponse::QueriedOrders(stock.get_sell_orders()))
                                    .await
                            } else {
                                market_tx.send(QueryResponse::SymbolNotFound).await
                            }
                        }
                    };

                    if let Err(e) = status {
                        market_speak(format!("Error: {:#?}", e), &mut stdout, true);
                    }
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    break;
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    break 'market_loop;
                }
            }
        }
    }

    if let Err(server_status) = server.await {
        eprintln!("Error: {:#?}", server_status);
    } else {
        println!("Bbye!");
    }
}

/// Prints a message to the terminal with a different colour for the market.
///
/// This colour is defined by `MARKET_OUTPUT_COLOUR`.
fn market_speak(message: String, stdout: &mut StandardStream, error: bool) {
    if let Err(e) = stdout.set_color(ColorSpec::new().set_fg(Some(MARKET_OUTPUT_COLOUR))) {
        println!(
            "Could not set terminal color({:#?}). The next statement is probably from the Market:",
            e
        );
    }

    if error {
        eprintln!("{}", message);
    } else {
        println!("{}", message);
    }

    if let Err(e) = stdout.reset() {
        println!(
            "Could not set terminal color({:#?}). Market output ends.",
            e
        );
    }
}
