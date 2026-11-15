#import "../lib/booklet.typ": *
#import "../lib/data.typ": *
#import "../lib/needs.typ": *
#import "../lib/care.typ": *
#import "../lib/social.typ": *
#chapter("ch-friends")[Friends, battles and stations]
#set text(size: 10pt)
#set par(leading: 0.6em, spacing: 0.9em)
#set list(spacing: 0.55em)

Pets are not alone at a hacker camp. This chapter covers the three things
that reach your pet from outside: the friends it meets over the mesh radio,
the battles it can fight with them, and the magic places that hand out a
treat when you tap the badge.

== Friends

#grid(columns: (1fr, auto), column-gutter: 5mm, align: top,
  [
    Every badge with this firmware joins a private mesh channel and sends out
    a small beacon from time to time. It carries the pet's kind, its name,
    its generation and its fighting numbers. When your badge hears another
    badge's beacon, that pet becomes a *friend*. Your own beacon, echoed
    back, is ignored.

    - *A new friend.* The toast says #w("new friend!"). The pet gains fun:
      #v("DOSE_FRIEND_BIG_FUN_PCT").
    - *A friend again.* After #v("FRIEND_BOOST_COOLDOWN") since the last
      boost, hearing the same friend gives #v("DOSE_FRIEND_SMALL_FUN_PCT") fun
      and the toast #w("+happy"). Sooner than that, the badge just notes
      that it saw them.
  ],
  hero-pic("happy"))

A tired pet gets the fun doubled, the same as for play (@ch-care). A sleeping
pet is not woken. A boost needs no menu and costs nothing. The badge must be on, with its
mesh radio running.

*The list.* The #w("Friends") row of the Stats menu opens it. It keeps the
last #v("FRIENDS_MAX") friends, the one seen most recently on top. When it is
full, the oldest drops off. The list is empty at first, and says
#w("No friends met yet"). Up and Down move the cursor. Fire opens the
details: the friend's name and kind, #w("Met <d>d <h>h ago") and
#w("Last seen <d>d <h>h ago"), four bars (#w("Atk"), #w("Def"), #w("Spd")
and #w("HP")) and the record between you two, as #w("Battles: <w>W-<l>L").
The foot of the list says #w("Fire for details"). The details card is
titled #w("Friend Details") and its corner says #w("Back"): any button goes
back. If the cursor has no friend under it, the card says
#w("No friend selected").

The "ago" times do not count the time the badge was off, so a friend can
look younger than they are.

== Battles

#versus()

*Where.* #w("Battle") is the first row of the Play menu, in both money modes.
It opens a list of your friends. The foot of the list says
#w("Fire to challenge"). The title bar shows your pet's own record,
#w("Battle <w>W-<l>L"): wins, then losses. The record belongs to this pet.
A new egg starts it at zero.

*How a fight works.* Press Fire on a friend. The fight is *instant*. Your
badge plays the whole thing at once, against the numbers that friend last
sent, so the friend does not need to be near. Each pet has four numbers:
attack, defense, speed and HP. They come from the care you gave. A
curious, well-kept pet hits harder. A lean, fit pet is quicker and harder to
hurt. A healthy pet has more HP. Each lasting condition, diabetic and
dependent, knocks a flat amount off attack, defense and speed. The faster
pet strikes first. Damage has a little luck in it. After a fixed number of
rounds, whoever has more HP left wins.

*The result.* A card with two HP bars and a banner,
#w("YOU WON!") or #w("YOU LOST"). The card is titled #w("Battle Result").
Your side is labelled #w("You") and the friend's side carries the friend's
name, or #w("Friend") when the name is empty. In money mode a win also shows
#w("+<n> HAX"), the reward of #v("BATTLE_HAX_REWARD"), and pays it. Just before the card, a short
full-screen picture shows both pets standing, and then the winner and the
loser. The card says #w("Any button to close"), and any button does. If no
fight has been played yet, it says #w("No result yet").

*It cannot hurt the pet.* Battle HP exists only for the length of the fight.
A loss changes the record and nothing else.

*The other badge.* The result is also sent out for the friend's badge to hear.
If it is in direct radio range, it counts the fight the other way round (your
win is its loss), starts its own #v("BATTLE_COOLDOWN") rest, replays the
picture, and pays its pet if that pet won and it plays with money. The
result does not travel across repeaters.

#tbl((auto, 1fr),
  table.header([*The picker says*], [*Because*]),
  [#w("No friends") and #w("to battle yet")], [The list is empty. Go and meet someone.],
  [#w("Friend not ready")], [That friend has not sent their fighting numbers yet. Try again later.],
  [#w("Can't battle now")], [The pet is busy or asleep. Wait.],
  [#w("On cooldown <n>s")], [The Battle rest has not run out. The count is in seconds.],
)

The Battle row itself is grey, and the cursor skips it, when the pet is
#w("asleep"), #w("busy"), #w("hibernating"), #w("still an egg") or
#w("pet is gone"). It says
#w("wait <time>") while the rest runs. An injured pet may still fight.

== Stations: the magic places

At events there are *stations*: a food stand, a medic and a rest corner.
Hold the back of the badge to the station's reader for a moment. The
reader sends a short signed phrase. The badge checks the signature, and
only then does the pet get its treat. A phone with a random NFC app
cannot do it. It has no key.

The badge ignores the tap unless it has a pet that has not gone. An egg
counts. A frozen pet counts too. The phrase is matched whatever its
capitals. Anything else is ignored without a word.

#station-row()

#tbl((auto, 1fr, auto),
  table.header([*Phrase*], [*What the pet gets*], [*Toast*]),
  [#raw("more food")],
  [Fullness and water lifted to #v("STATION_FOOD_FLOOR_PCT"), never lowered.],
  [#w("Food bonus!")],
  [#raw("more drugs")],
  [Health up to the pet's own ceiling. Booze down to at most
   #v("STATION_BOOZE_CAP_PCT"). Hangover, injury and groggy cleared.
   Dependence and diabetes stay.],
  [#w("Heal bonus!")],
  [#raw("sleep like a bear")],
  [Energy full. Groggy and worn out cleared. The next waking forgets any
   oversleeping, so it cannot be groggy. It does not put the pet to sleep.],
  [#w("Sleep bonus!")],
)

*The cooldown.* Each phrase rests for #v("STATION_COOLDOWN"). The first tap
after a boot always works. A tap too soon does nothing to the pet. It shows a
countdown in the toast corner, like #raw("wait 4:50"). A reboot clears the
rests.
