Story Stuff Ideas
=================

A lot of this shouldn't be infodumped, but should instead be sprinkled through the world or something?

## How does time travel work, anyway?

* Time travel is actually pretty easy, *if you can set up a stable time loop*
* Whether or not a portal opens is determined by whether or not you will be able to perfectly execute the loop.
* Doing so is basically impossible without the ability to predict the future
* Maybe prediction spell is paired with precommitment thing that executes the planned actions.
* Something about auxiliary spells handling the microscopic effects?

## Magical guilds

Say the practice of magic is divided into specialties, each of which has its own guild.
The guilds are a bit competative with each other when their specialties offer different solutions to the same problem.
The time travel guild is kind of dumb because they can't actually use time loops.
Maybe they can send messages to the future or something, but as far as they're concerned time loops are impossible.

You are *not* from the time guild: You're from the perception guild or something?
They specalize in scrying, which turns out to be the actual hard pert of time travel.
You have two scrying spells:
* Spatial scrying (why you can see through walls)
* Temporal scrying (how you can predict the future.
Probably "all in your head": using the results of spatial scrying to predict the (local) future.
Increased cost as you predict farther out, since you need exponentially higher resolution.

Indistinguishable Objects
=========================

(Define loop free coloring)

When all the portals are closed, we want to ensure that the portal graph has a loop-free coloring.
But more than that, we'd like to ensure that the player is never "doomed":
they're never in a position where the game won't let them advance because it would force a loop coloring.
Surprisingly, we get this for free by just enforcing the loop-free coloring condition online.
To prove this, we need to show that every graph with a loop-free coloring
can be legally modified to have no open portals,
and still posess a loop-free coloring.
If a coloring is loop-free, all we need to do to preserve that is avoid connecting a worldline to itself.
If you want to close a portal,
you need to connect the ends of worldlines to the start of the worldlines originating from the portal.
This will always be possible as long as the worldline's end isn't the only end that exists.
But if any worldlines originate from Start, this will never happen.
In other words, if there were any original instances of the object,
and there exists a loop-free coloring, the player is not doomed.

We'd like to defer adding hypothetical objects to the graph
until they're reified by closing their spawn portal.
Say there exists a valid coloring if you track hypothetical objects.
What happens when you remove the hypothetical objects?
Obviously removing worldlines cannot introduce a loop: so far so good.
Say you have a loop-free coloring without the hypothetical worldlines drawn in.
Drawing in the hypothetical worldlines does two things:
it creates new worldlines fromn the portal to the player's current location.
This obviously does not create loops, since the player's worldline is loop-free.
It also traces the worldlines of dropped objects back to the portal.
This cannot create loops, since the portal has no incomming worldlines (it's still open).

Therefore, the hypothetical-free graph has a loop-free coloring iff the full graph has one.

When the hypothetical player is reified, can we recover the hypothetical worldlines?
This seems streightforward: we draw the correct number of worldlines out of the portal,
terminating one whenever we encounter a node where an item "spawned" from a hypothetical object.

Say every vertex in the portal graph has a path to Escape,
yet the current "coloring" of worldlines has loops.
Pick a loop. It must touch another worldline, either a loop or an escaping line.
If it doesn't touch either, it won't be connected to escape at all.
In either case, you can "splice" the loop into the other line, and reduce the count of loops by 1.
Iterating on this procedure removes all the loops.
This means that there always exists a loop-free coloring of the portal graph
when every vertex has a path to Escape.

Thus far, we've assumed an edge for every object.
Since at this point we only care about graph connectivity,
we can simplify our representation by merging adjacent edges.

We'd also like to be able to check that the entire graph is connected to escape as cheaply as possible.
But we don't want to recheck the entire graph every time.
If we assume that our current state is fully connected to Escape,
we can use that information to check our sucessor state more easily.
When we alter the portal graph,
we're taking some edges that used to go to Escape and pointing them somewhere else.
If wherever those edges are now pointing can reach Escape,
every node that transitively relied on them is still able to reach Escape.
This lets us check the continued connectivity of the portal graph to Escape
without rechecking the entire graph.

Player
======

The player has propeties (health? mana?) that vary over time, and have a lower and upper bound. Since only one player can go through the portal at a time, there's no ambiguity, and the constraints can be propagated pretty streightforwardly.

Tools and such
==============

Say you appear with 2 of the same sword, which has a finite number of uses. The issue is ordering them: Say your graph looks like:

s -0-> 1
1 -1-> 1
1 -2-> 1
1 -3-> e

There's a period where 3 swords exist: the original 1 and the 2 from the portal. THe lifetime of the sword could look like:

0 -> 1 -> 2 -> 3
0 -> 2 -> 1 -> 3
Not valid, but maybe we don't know that?
0 -> 2 -> 3
1 -> 1

If everything is commutative, this is ok, since they can all be concatenated together at the end. If not, you're sad. Maybe make the user deal with it? Probably ignore the problem for now.



