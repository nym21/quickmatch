use quickmatch::{QuickMatch, QuickMatchConfig};

fn main() {
    let content = include_str!("../metrics.txt");
    let metrics: Vec<&str> = content.lines().collect();
    let qm = QuickMatch::new(&metrics);
    let config = QuickMatchConfig::new().with_limit(10);

    let queries = [
        "price", "price close", "price-close", "realized price",
        "supply", "mvrv", "sopr", "sth", "lth", "utxo",
        "hash rate", "market cap", "realized cap", "tx count",
        "fee rate", "block size", "hashrate", "marketcap", "feerate", "blocksize",
        "pric", "suply", "realizd", "hashrat", "mvr",
        "real", "profit", "loss", "vol", "cap", "sent",
        "active", "dormant", "sth supply", "lth realized",
        "p2pkh supply", "p2tr sent", "nupl", "nvt", "cvdd", "puell", "rhodl",
        "difficulty", "coinbase", "vbytes", "segwit", "taproot",
        "short term holder", "long term holder supply",
        "realized market cap", "unrealized profit", "net unrealized",
        "bitcoin price usd", "total supply", "number of transactions",
        "transaction count", "mining revenue", "block reward",
        "average fee", "median fee rate", "MarketCap", "MVRV", "HashRate",
        "SOPR", "NUPL", "hodl", "pnl", "roi", "ath", "drawdown",
        "volatility", "dominance", "inflation", "velocity", "thermocap",
        "sma 200", "sma200", "30d", "1year", "return 1y", "realised",
        "sth/supply", "lth.mvrv",
        "dom", "sub dom", "fee dom", "do", "d", "s",
        "sma_200", "200 sma", "cap market", "rate hash",
        "supply xyz", "xyz abc",
    ];

    for query in &queries {
        let results = qm.matches_with(query, &config);
        println!("{}\t{}", query, results.join(","));
    }
}
