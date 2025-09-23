mod types;

use std::collections::HashMap;
use std::env;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};
use tokio::sync::mpsc::error::SendError;
use tokio::{select, signal, task, time};
use types::{Market, Query, QueryResponse, Stock};

const TICK_INTERVAL_MILLISECS: u64 = 10;
const MARKET_OUTPUT_COLOUR: Color = Color::Yellow;

#[tokio::main]
async fn main() {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    let (server_tx, mut market_rx) = mpsc::channel::<(usize, Query)>(32);

    let mut market = Market::new();
    let initial_stocks = vec![Stock::new("V", "Vulyenne")];
    market.extend_stocks(initial_stocks.into_iter());

    let mut tick_interval = time::interval(time::Duration::from_millis(TICK_INTERVAL_MILLISECS));
    tick_interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    let mut cmd_args = env::args();
    let mut listener_address = String::from("127.0.0.1:8080");

    while let Some(arg) = cmd_args.next() {
        if arg == "-p" {
            if let Some(url) = cmd_args.next() {
                listener_address = url;
            }
        }
    }

    // a unique ID is mapped to each connection
    let mut connections: HashMap<usize, mpsc::Sender<QueryResponse>> = HashMap::new();
    market_speak(format!("Starting server at {}. Press Ctrl+C to shut down.", &listener_address), &mut stdout, false);
    let server = task::spawn(serve(server_tx, listener_address));

    'market_loop: loop {
        tick_interval.tick().await;
        loop {
            let executed_trades = market.resolve();

            for (symbol, trades) in executed_trades.into_iter() {
                for trade in trades.into_iter() {
                    market_speak(
                        format!("Market says> Trade executed for {}: {:#?}", symbol, &trade),
                        &mut stdout,
                        false,
                    );

                    if let Some(buyer_tx) = connections.get(&trade.buyer_id) {
                        if let Err(e) = buyer_tx.send(QueryResponse::ExecutedTrade(trade)).await {
                            market_speak(
                                format!("Error while sending trade to buyer: {:#?}", e),
                                &mut stdout,
                                true,
                            );
                        }
                    } else {
                        market_speak(
                            format!("Buyer with id {} not connected.", trade.buyer_id),
                            &mut stdout,
                            true,
                        );
                    }

                    if let Some(seller_tx) = connections.get(&trade.seller_id) {
                        if let Err(e) = seller_tx.send(QueryResponse::ExecutedTrade(trade)).await {
                            market_speak(
                                format!("Error while sending trade to seller: {:#?}", e),
                                &mut stdout,
                                true,
                            );
                        }
                    } else {
                        market_speak(
                            format!("Seller with id {} not connected.", trade.seller_id),
                            &mut stdout,
                            true,
                        );
                    }
                }
            }

            match market_rx.try_recv() {
                Ok((id, query)) => {
                    let status = resolve_query(id, query, &mut connections, &mut market, &mut stdout).await;
                    if let Err(e) = status {
                        market_speak(format!("Error: {:#?}", e), &mut stdout, true);
                    }
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    break;
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    market_speak("Server disconnected, market shutting down.".to_string(), &mut stdout, false);
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

async fn resolve_query(id: usize, query: Query, connections: &mut HashMap<usize, mpsc::Sender<QueryResponse>>, market: &mut Market, stdout: &mut StandardStream) -> Result<(), SendError<QueryResponse>> {
    // If there is a new connection, add it, otherwise check if the ID exists first.
    let socket_tx = match query {
        Query::Connect(socket_tx) => {
            connections.insert(id, socket_tx);
            let t = connections.get(&id).expect("This key was just added, it must exist.");
            t.send(QueryResponse::Connected).await?;
            return Ok(());
        }
        _ => {
            match connections.get(&id) {
                Some(socket_tx) => socket_tx,
                None => {
                    market_speak(format!("Query from unknown id {}.", id), stdout, true);
                    return Ok(());
                }
            }
        }
    };

    match query {
        Query::Connect(_) => {
            unreachable!("Connection should already have been handled.");
        }
        Query::Buy(symbol, order) => {
            if let Some(stock) = market.get_stock_mut(&symbol) {
                stock.add_buy_order(order);
                socket_tx.send(QueryResponse::OrderPosted).await?;
            } else {
                socket_tx.send(QueryResponse::SymbolNotFound).await?;
            }
        }
        Query::Sell(symbol, order) => {
            if let Some(stock) = market.get_stock_mut(&symbol) {
                stock.add_sell_order(order);
                socket_tx.send(QueryResponse::OrderPosted).await?;
            } else {
                socket_tx.send(QueryResponse::SymbolNotFound).await?;
            }
        }
        Query::Ohlc(symbol) => {
            if let Some(stock) = market.get_stock(&symbol) {
                let (open, high, low, close) = stock.get_ohlc();
                socket_tx.send(QueryResponse::Ohlc(open, high, low, close)).await?;
            } else {
                socket_tx.send(QueryResponse::SymbolNotFound).await?;
            }
        }
        Query::BuyOrders(symbol) => {
            if let Some(stock) = market.get_stock(&symbol) {
                socket_tx.send(QueryResponse::QueriedOrders(stock.get_buy_orders())).await?;
            } else {
                socket_tx.send(QueryResponse::SymbolNotFound).await?;
            }
        }
        Query::SellOrders(symbol) => {
            if let Some(stock) = market.get_stock(&symbol) {
                socket_tx.send(QueryResponse::QueriedOrders(stock.get_sell_orders())).await?;
            } else {
                socket_tx.send(QueryResponse::SymbolNotFound).await?;
            }
        }
    }

    Ok(())
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

pub async fn serve(tx: mpsc::Sender<(usize, Query)>, listener_address: String) -> Result<(), std::io::Error> {
    let mut next_id = 1;
    let mut connection_future_set = task::JoinSet::new();
    
    let listener = TcpListener::bind(listener_address).await?;

    let (shutdown_signal_tx, shutdown_signal_rx) = watch::channel(false);
        
    loop {
        select! {
            sigint = signal::ctrl_c() => {
                if let Err(e) = sigint {
                    eprintln!("Error while waiting for ctrl-c: {:#?}, stopping server.", e);
                }
                break;
            }

            socket_result = listener.accept() => {
                let (mut socket, _) = match socket_result {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Error while accepting connection: {:#?}", e);
                        continue;
                    }
                };
        
                let conn_id: usize = next_id;
                next_id += 1;
        
                let (socket_tx, socket_rx) = mpsc::channel::<QueryResponse>(32);
        
                if let Err(e) = tx.send((conn_id, Query::Connect(socket_tx))).await {
                    eprintln!("Encountered error while sending {:#?}", e);
                    if let Err(e) = socket.shutdown().await {
                        eprintln!("Error while shutting down socket: {:#?}", e);
                    }
                    continue;
                }
        
                connection_future_set.spawn(connection_handler(conn_id, tx.clone(), socket_rx, socket, shutdown_signal_rx.clone()));
            }
        }
    }

    match shutdown_signal_tx.send(true) {
        Ok(()) => {
            let results = connection_future_set.join_all().await;
            for result in results {
                if let Err((id, e)) = result {
                    eprintln!("Connection with id {} returned error: {:#?}", id, e);
                }
            }
        },
        Err(e) => {
            eprintln!("Error while sending shutdown signal: {:#?}, forcing shutdown on sockets.", e);
            
            // I don't see a need to manually shutdown the sockets here.
            // connection_future_set.shutdown().await;
        },
    }

    Ok(())
}

async fn connection_handler(id: usize, tx: mpsc::Sender<(usize, Query)>, mut rx: mpsc::Receiver<QueryResponse>, mut socket: TcpStream, mut shutdown_signal: watch::Receiver<bool>) -> Result<(), (usize, std::io::Error)> {
    let mut socket_buffer = [0u8; 1024];
    loop {
        select! {
            query_response = rx.recv() => {
                let response = match query_response {
                    Some(r) => r,
                    None => {
                        // The market should not be closed before sockets.
                        socket.write(r#"{"response": "market closed"}"#.as_bytes()).await.map_err(|e| (id, e))?;
                        socket.shutdown().await.map_err(|e| (id, e))?;
                        continue;
                    }
                };
        
                let response = response.to_json();
                if let Err(e) = socket.write_all(response.as_bytes()).await {
                    eprintln!("Error while writing to socket: {:#?}", &e);
                    break Err((id, e));
                }
            }
            socket_query = socket.read(&mut socket_buffer) => {
                let message = String::from_utf8_lossy(&socket_buffer);
                println!("Received: {}", message);
                let query = match socket_query {
                    Ok(0) => {
                        break Ok(());
                    }
                    Ok(n) => {
                        match Query::from_json(&message[0..n], id) {
                            Some(q) => q,
                            None => {
                                socket.write(r#"{"response": "malformed request"}"#.as_bytes()).await.map_err(|e| (id, e))?;
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error while reading from socket: {:#?}", e);
                        break Err((id, e));
                    }
                };
        
                if let Err(e) = tx.send((id, query)).await {
                    eprintln!("Error while sending query: {:#?}", e);
                    break Ok(());
                }
            }
            _ = shutdown_signal.changed() => {
                socket.shutdown().await.map_err(|e| (id, e))?;
                break Ok(());
            }
        }
    }
}

