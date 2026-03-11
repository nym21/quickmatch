import { QuickMatch, QuickMatchConfig } from "../src/index.js";

// Case 1: Items with colon, query with colon (default separators)
const items1 = ["word: word2", "other: thing", "word word2"];
const qm1 = new QuickMatch(items1);
console.log("Default seps | query 'word: word2' vs items with colon:", qm1.matches("word: word2"));

// Case 2: Query "word:" against items
const items2 = ["word: word2", "word word2", "word", "word:"];
const qm2 = new QuickMatch(items2);
console.log("Default seps | query 'word:' vs mixed items:          ", qm2.matches("word:"));

// Case 3: Colon as separator, query "word: word2"
const config3 = new QuickMatchConfig().withSeparators("_- :");
const items3 = ["word: word2", "other: thing", "word word2"];
const qm3 = new QuickMatch(items3, config3);
console.log("Colon as sep | query 'word: word2':                   ", qm3.matches("word: word2"));

// Case 4: Colon as separator, query "word:"
const config4 = new QuickMatchConfig().withSeparators("_- :");
const items4 = ["word: word2", "word word2", "word", "word:"];
const qm4 = new QuickMatch(items4, config4);
console.log("Colon as sep | query 'word:' vs mixed items:          ", qm4.matches("word:"));
