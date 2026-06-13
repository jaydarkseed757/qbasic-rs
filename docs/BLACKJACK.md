# How Blackjack Deluxe VGA Works

A deep-dive into the code structure, design decisions, and QBASIC-specific techniques used in `BLACKJCK.BAS`.

---

## Overall architecture

The file is a single ~1140-line QBASIC 1.1 program. QBASIC requires all `SUB` and `FUNCTION` procedures to be declared before the main body, so the file opens with 45 `DECLARE` statements. The module-level code (the `TYPE` definitions, the shared `DIM` block, and the startup body that runs top-to-bottom from `RANDOMIZE TIMER` to `END`) sits above the first procedure definition. Everything below that is subprograms, which QBASIC hoists and compiles before execution begins.

The code is divided into ten labelled sections separated by comment banners:

```
INITIALIZATION → GAMEPLAY → MONEY → CARDS → GRAPHICS → DEALER → SCREENS → STATISTICS → UTILITIES → SOUND
```

Global state is held in `DIM SHARED` variables accessible from every SUB/FUNCTION — QBASIC has no modules or namespaces, so this is the idiomatic way to share state without passing every variable as a parameter.

---

## DEFINT A-Z and the type system

The very first executable line is `DEFINT A-Z`. This tells QBASIC that any variable whose name starts with any letter defaults to INTEGER (16-bit signed, −32768 to 32767) rather than SINGLE (floating point). This is a common QBASIC optimization: integer math is faster and less likely to produce floating-point surprises.

The exceptions are called out explicitly:
- `bankroll`, `bet`, and `amt&` are LONG (32-bit, suffix `&`) — they hold dollar amounts that could exceed $32,767 after a winning streak
- Time variables use SINGLE with the `!` suffix: `sec!`, `t!`
- String variables use `$` suffix as normal: `k$`, `s$`, `rn$`

Because of `DEFINT A-Z`, every SUB/FUNCTION parameter is implicitly INTEGER unless the suffix says otherwise. The `DECLARE` statements must match: `Settle (outcome%, amt&, m$)` is explicit about the LONG and the string.

---

## Card encoding

Cards are integers 1–52:

```
rank = ((c - 1) MOD 13) + 1     ' 1=Ace, 2-10, 11=Jack, 12=Queen, 13=King
suit = (c - 1) \ 13             ' 0=Hearts, 1=Diamonds, 2=Clubs, 3=Spades
```

This encoding packs both rank and suit into a single integer, which matters because QBASIC arrays of INTEGER are the most memory-efficient structure available. The `\` operator is integer division (truncating), `MOD` is remainder — both are integer operations that work cleanly here.

Suits 0 and 1 (Hearts, Diamonds) are red (color 4); suits 2 and 3 (Clubs, Spades) are black (color 0). This falls out naturally: `IF s < 2 THEN clr = 4 ELSE clr = 0`.

---

## The deck and shuffle

`ShuffleDeck` initializes `deck(1..52)` with values 1–52 then runs a Fisher-Yates shuffle:

```basic
FOR i = 52 TO 2 STEP -1
   j = INT(RND * i) + 1
   SWAP deck(i), deck(j)
NEXT
```

`deckPos` is a shared pointer into the array. `NextCard` increments it and returns the next card. A reshuffle is triggered at the start of `PlayHand` when `deckPos > 30` (roughly 22 cards remaining) — this mimics a casino's cut card and prevents the deck running out mid-hand.

`RANDOMIZE TIMER` seeds the RNG once at startup using the system clock. QBASIC's `RND` is a linear congruential generator — good enough for a game, not for cryptography.

---

## Hand value calculation

`HandValue(who)` handles the soft/hard ace rule cleanly:

1. Count all aces as 11 and accumulate `aces`
2. After summing, while the total exceeds 21 and there are aces, subtract 10 and decrement `aces`

This is O(n) in the number of cards and handles any number of aces correctly — e.g. A+A+9 = 11+11+9=31 → drop one ace → 21.

```basic
DO WHILE t > 21 AND aces > 0
   t = t - 10
   aces = aces - 1
LOOP
```

---

## The main game loop

The outermost loop (`DO WHILE playing = 1`) handles full play sessions. After the title screen, `ChooseStake` runs before the inner hand loop — the player picks a table (Low / Medium / High Roller), which sets the starting `bankroll`, `tblMin`, `tblMax`, and `st.best`. Inside it, a second loop runs hands until the bankroll hits zero or the player quits. When the bankroll reaches zero, `GameOver` is called — it returns 1 (play again, re-entering `ChooseStake`) or 0 (exit). This two-level loop structure means a "play again" resets stats and bankroll cleanly without restarting the program.

---

## PlayHand — the hand state machine

`PlayHand` is the core of the game. It runs one complete hand in a straight-line sequence:

1. **Reshuffle check** — `deckPos > 30`
2. **Bet** — `GetBet`
3. **Deal** — four `DealOne` calls in casino order: player, dealer, player, dealer (hole card hidden)
4. **Insurance** — offered only when dealer upcard is an Ace (`upRank = 1`) and player can afford it (`bankroll >= bet + bet\2`)
5. **Dealer blackjack check** — if dealer has 21, reveal hole card, pay insurance, settle immediately
6. **Player blackjack check** — pays 3:2 via `Settle 2, bet * 3 \ 2, "Blackjack!"`
7. **Player turn loop** — runs until stand/double/surrender/bust/21
8. **Dealer turn** — reveal hole card, draw until `HandValue(2) >= 17`
9. **Payout** — compare totals, call `Settle`

The `first` flag tracks whether it's the player's first action — double down and surrender are only valid on the first action, so the prompt text and input validation both check `first = 1`.

---

## Settle — the single payout choke point

All outcomes funnel through `Settle(outcome, amt&, m$)`:

| outcome | meaning | bankroll effect |
|---------|---------|----------------|
| 2 | blackjack | +amt& |
| 1 | win | +amt& |
| 0 | push | none |
| -1 | loss / surrender | -amt& |
| -2 | bust | -amt& |

`Settle` updates `bankroll`, increments the right stat counter, triggers the outcome sound, calls `DealerSay` with the result message, and refreshes the status bar. Nothing else touches these — this is the design constraint that keeps outcomes consistent.

Note that insurance is handled outside `Settle` in `PlayHand` directly, because it settles before the main hand result and at a different amount.

---

## The bitmap font (BigLetter / BigText)

QBASIC's `PRINT` statement in SCREEN 12 draws characters with a solid black cell background. On the status bar or speech box this is fine, but on a white card face it would paint a black rectangle around each letter — ugly.

The solution is a hand-coded 5×7 bitmap font. Each character is stored as seven integers (one per row), where each integer's bits encode which of the five columns are lit. For example, "A":

```
p(1) = 14   = 01110  →  _XXX_
p(2) = 17   = 10001  →  X___X
p(3) = 17   = 10001  →  X___X
p(4) = 31   = 11111  →  XXXXX
p(5) = 17   = 10001  →  X___X
p(6) = 17   = 10001  →  X___X
p(7) = 17   = 10001  →  X___X
```

`BigLetter` reads each bit with `AND` and draws a filled rectangle for each set pixel. The scale parameter `sc` multiplies each pixel — at `sc=1` the font is 5×7 pixels (used for card corner ranks), at `sc=6` it produces the 30×42 pixel title text. The background is never touched, so the font is transparent.

`BigText` advances the x cursor by `6 * sc` per character (5 pixel wide + 1 pixel gap), skipping spaces.

---

## Card drawing (DrawCard / DrawBackCard / DrawSuit)

**DrawCard** at `(x, y)` for card `c`:
1. Fills a 56×80 white rectangle
2. Draws a black border, then clears the four corner pixels (visual rounded-corner effect)
3. Draws the rank label twice in the top-left and bottom-right corners using `BigText` at scale 1
4. Draws a small suit symbol in the corner using `DrawSuit`
5. For face cards (J/Q/K), draws the large rank letter at scale 3 in the center; for number cards, draws one large center suit symbol

**DrawBackCard** draws a blue card with a diamond grid pattern — two nested loops draw four `LINE` segments per grid point forming a ◇ shape.

**FlipHoleCard** animates the hole-card reveal. It runs two mirror-image loops over a `strip` variable from 4 to 28 in steps of 4 (half the card's 56-pixel width). Phase 1 overdaws felt-green (`LINE ... BF`) rectangles of increasing width on both the left and right edges of the card, making the visible back-card width shrink to zero — simulating the card rotating edge-on. At the pivot the face card is drawn underneath the full green covers, a short click sound (`SOUND 600, 1`) fires, then phase 2 runs the loop in reverse, removing the green from the outside in until the face is fully exposed. All three hole-card reveal sites — dealer blackjack, player bust, and normal dealer turn — call `FlipHoleCard` instead of calling `DrawCard` directly.

**DrawSuit** draws filled suit symbols using QBASIC graphics primitives:
- **Hearts** — two overlapping circles filled with PAINT, then a downward triangle filled by painting the center
- **Diamonds** — four LINE segments forming a ◇, then PAINT to fill
- **Clubs** — three overlapping circles plus a short stem rectangle
- **Spades** — inverse of hearts (two circles below pointing up, triangle pointing up), plus a stem

`PAINT` fills an enclosed region with a color, flood-fill style. It's used here instead of drawing complex filled polygons directly.

---

## Dealing animation (AnimateDeal)

`AnimateDeal(dx, dy)` slides a card back from the deck pile at `(568, 224)` to the destination slot `(dx, dy)` in two phases:

1. **Horizontal slide** — move left from x=568 to x=dx in steps of 24 pixels, erasing the previous position with a green rectangle and redrawing the card back
2. **Vertical slide** — move up or down from y=224 to y=dy in steps of 24 pixels, same erase-and-redraw

After the card arrives, `DrawBackCard 568, 224` redraws the deck pile (the animation erased it during the horizontal phase). The 24-pixel step and 15ms delay were tuned to look smooth without being slow.

All animation happens in the y=224 "lane" between the dealer row (y=128) and player row (y=304) — this clear strip means the sliding card doesn't clip any placed cards.

---

## Bet chip display (DrawBetChips)

`DrawBetChips(amt&)` visualizes the current wager as physical casino chips. It performs a greedy denomination breakdown of the bet into 500 / 100 / 25 / 5 / 1 values (purple / black / green / red / white chips), then draws one vertical stack per denomination present, packed left to right, with the denomination value printed beneath each stack:

```basic
due& = amt&
FOR d = 1 TO 5
   n& = 0
   DO WHILE due& >= dv(d)
      due& = due& - dv(d)
      n& = n& + 1
   LOOP
   ...draw a stack of min(n&, 8) chips...
NEXT d
```

Each chip is a flattened ellipse (`CIRCLE ... aspect .4`) filled with `PAINT` and outlined with a contrasting rim color. Stack height is the chip count (visually capped at 8 — the status bar always shows the exact dollar total).

The display lives in the felt box `(486,306)-(630,418)` — deliberately placed **below** the deck pile (which ends at y=304) and **right** of the player card row. This is the one open felt region the dealing animation never crosses, so the chips are never clipped or erased mid-deal. `DrawBetChips` is called from `GetBet` once a wager is accepted and again from the double-down path after the bet doubles. `ClearTableArea` wipes the box at the start of each hand, so no stale chips carry over.

Note `due&` is named to avoid `rem` — `REM` is the BASIC comment keyword and cannot be used as a variable.

---

## Screen layout and the persistence model

SCREEN 12 is 640×480 pixels. The screen is divided into zones with different persistence:

```
y=0   - y=111   Permanent: dealer art, title, speech box
y=112 - y=424   Cleared per hand: ClearTableArea() — green felt, cards, totals
y=425 - y=430   Gold separator line
y=431 - y=479   Permanent: black status bar (bankroll, bet, prompt)
```

`DrawTable` is called once per session (not per hand). `ClearTableArea` is called at the start of each hand and restores the deck pile. This means the dealer, title, and status bar never need to be redrawn, which keeps `PlayHand` simple.

Text positions use QBASIC's `LOCATE row, col` (1-based, 80×30 character grid mapped onto the 640×480 pixel screen). Key positions:
- Row 3: dealer speech text
- Row 8: dealer total
- Row 25: player total
- Row 28: bankroll / bet
- Row 29: action prompt

---

## The dealer character

`DrawDealer(cx, fy, expr)` draws a cartoon dealer using only LINE, CIRCLE, and PAINT. It reads the global `dealerType` to pick hair and tie colors, then draws facial features that vary by expression.

**Dealer types** — randomized once per session via `dealerType = INT(RND * 3)`:

| Type | Name | Hair color | Tie color | Extra |
|------|------|-----------|-----------|-------|
| 0 | Mike | Brown (6) | Red (4) | — |
| 1 | Sandy | Blonde (14) | Blue (9) | — |
| 2 | Frank | Dark gray (8) | Purple (5) | Glasses |

Frank's glasses are drawn last (after the eyes) as two `LINE ... B` rectangles with a bridge line, so they overlay the eye circles correctly.

**Expressions** — set by `PlayHand` just before each `Settle` call:

| expr | Situation | Eyes | Eyebrows | Mouth |
|------|-----------|------|----------|-------|
| 0 | Neutral / start of hand / push | Normal circles | Straight | Smile arc r=8 |
| 1 | Dealer wins / player busts | Normal | Raised flat | Bigger smile arc r=9 |
| 2 | Dealer busts / player blackjack | Wide circles (r=3) | Outer corners raised | Filled O-mouth circle |
| 3 | Dealer blackjack win | Wink (right eye = line) | Angled outward-up | Wide grin arc r=11 |
| 4 | Player wins | Normal | Drooping inward | Frown (two LINE segments forming a V) |

The character is drawn relative to `(cx, fy)` so it works anywhere — the title screen uses `DrawDealer 120, 220, 0` and gameplay uses `DrawDealer 320, 48, expr`. Because the dealer art lives above y=112 and `ClearTableArea` only wipes y=112–424, expressions persist across the hand until explicitly redrawn.

---

## Sound design

All sound uses QBASIC's `SOUND freq, duration` command (PC speaker). Duration is in clock ticks (18.2/second). The sounds are:

| Event | Design |
|-------|--------|
| Deal | Short descending two-tone (880→660 Hz) — a quick "snap" |
| Chip | Rising two-tone (1320→1760 Hz) — bright and positive |
| Win | Rising three-note fanfare (C5→E5→G5) |
| Lose | Descending two-note fall (E4→C4) |
| Bust | Descending three-note droop (A3→E3→A2) |
| Blackjack | Win fanfare plus high C6 held longer |
| Push | Flat repeated note (A4 twice) — neutral |
| Shuffle | Rising scale 400→900 Hz in 100 Hz steps |

The title screen plays a looping 16-note casino-jazz phrase using `PLAY "MB ..."` (music background mode). The melody is stored in a local string `tune$` and started once before the "PRESS ANY KEY" blink loop. Inside the loop the program checks `PLAY(0)` — the number of notes remaining in QBASIC's background music buffer — and refills with another `PLAY "MB " + tune$` call when fewer than five notes remain, ensuring seamless looping. When the player presses a key the program continues; any residual buffered notes drain naturally before the first `SOUND` call from gameplay.

---

## Input handling

`GetKey$` spins on `INKEY$` until a key is pressed, then returns it uppercased. QBASIC's `INKEY$` is non-blocking (returns `""` if no key is waiting), so the spin loop burns CPU — acceptable for a single-tasking DOS program.

`KeyWait(sec!)` adds a timeout: spin for up to `sec!` seconds, return 1 if a key was pressed or 0 if timed out. This powers the blinking "PRESS ANY KEY" on the title screen — the loop alternates between bright yellow (color 14) and dark grey (color 8), each iteration calling `KeyWait` for the blink half-period.

`GetNum&` handles numeric bet entry character by character: digits are appended to a string and echoed, backspace (`CHR$(8)`) removes the last character and backs up the cursor with `CHR$(29)` (the QBASIC cursor-left code). Enter is accepted at any point — including immediately with an empty string, in which case `GetNum&` returns 0. `GetBet` treats a 0 return as "player hit Enter with no input" and defaults the bet to `tblMin`, printing the value at the cursor so the player sees it before the deal. This lets experienced players skip digit entry and just hit Enter to bet the minimum each hand. This approach avoids `INPUT`, which would let the player type arbitrary text and would scroll the screen.

---

## Midnight rollover in Delay

`Delay(sec!)` uses `TIMER` (seconds since midnight as SINGLE). The naive approach — `LOOP UNTIL TIMER - t! >= sec!` — breaks at midnight when TIMER resets to 0, producing a huge negative delta. The fix:

```basic
DO
   IF TIMER < t! THEN EXIT DO   ' midnight: bail out early
LOOP UNTIL TIMER - t! >= sec!
```

This exits immediately on rollover rather than hanging. An imperfect delay at midnight is fine for a game.

---

## Table selection (ChooseStake)

`ChooseStake` runs once at the start of each session (and again after a "play again" from the bust screen). It presents three numbered table options and waits for key 1, 2, or 3:

| Key | Table | Starting bankroll | Bet limits |
|-----|-------|------------------|------------|
| 1 | Low Stakes | $500 | $5 – $50 |
| 2 | Medium | $1,000 | $5 – $100 |
| 3 | High Roller | $5,000 | $25 – $500 |

On selection it sets four shared variables — `bankroll`, `tblMin`, `tblMax`, and `st.best` — then highlights the chosen row in yellow and plays `SndChip`. All subsequent bet validation in `GetBet` uses `tblMin`/`tblMax` instead of hardcoded limits, and `ShowStatus` displays the active range on the status bar.

---

## High score persistence (HISCORE.DAT)

The top-5 table is stored in a random-access binary file `HISCORE.DAT` alongside `BLACKJCK.BAS`. Each record is a `HiEntry` TYPE:

```basic
TYPE HiEntry
   nm    AS STRING * 3   ' 3-char initials
   score AS LONG         ' final bankroll at cash-out
   hands AS INTEGER      ' hands played that session
END TYPE
```

Record size = 3 + 4 + 2 = 9 bytes. `OPEN "HISCORE.DAT" FOR RANDOM AS #1 LEN = 9` then `GET`/`PUT` at record positions 1–5. `LOF(1)` returns total file bytes; `i * 9 > LOF(1)` guards against reading past end-of-file on first run.

**Flow:** `LoadHiScores` runs once at startup. After the player cashes out, `CheckHiScore` compares `bankroll` against the five stored scores (sorted descending). If it qualifies, the player enters 3-char initials via a simple `INKEY$` loop, the table is shifted down from the insertion point, the new entry is inserted, and `SaveHiScores` writes all five records back. `ShowHiScores` then draws the full table with `BigText "HALL OF FAME"` and color-coded rows. It is also called at the start of `TitleScreen` if the table is non-empty, giving the classic DOS game "hall of fame splash before the main title" feel.

Only `FarewellScreen` sessions (player cashes out with money) are eligible for the table — bankrupt sessions (`GameOver`) are not recorded since the final bankroll is always zero.

---

## What's not implemented

- **Split pairs** — would require a second hand array (`pHand2`) and a second player turn loop in `PlayHand`, plus UI to show two hands simultaneously. The biggest gap in casino rules.
- **Multiple decks** — a 2/4/6-deck shoe with a visible cut card; would change `ShuffleDeck` to fill a larger array and adjust the reshuffle threshold
- **Side bets** — Perfect Pairs or 21+3, wired through `Settle`
- **Basic strategy hint mode** — optional "book" play shown before the player acts
- **`PLAY`-based in-game music** — background music plays only on the title screen; in-game sounds use `SOUND` only
- **Card flip animation for new cards** — `FlipHoleCard` animates the hole-card reveal, but hit/deal animations still show the card back sliding in without a face-reveal flip
- **Status-bar win/loss flash and a reshuffle riffle animation** — small polish items still on the list
