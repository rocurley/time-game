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

Say that we only have indistinguishable objects here. Over time, we see:

1 -> 2
2 -> 2
2 -> 3

This can be "unrolled" in two different ways:

1 -> 2 -> 3 , 2 -> 2 (no escape for 2nd one)
1 -> 2 -> 2 -> 3 (escapes to 3)

How can we tell these apart?

For every object that comes out of a portal, we have an edge in and an edge out.
Imagine each edge as having a "color" corresponding its identity. We want to color the edges so that every node (except the start and end points) have the color coming in the same number of times as out. You can't have any loops, which I guess means that every color trail has to escape.

Obviously the correct number of color trails is the number of the thing that existed at the start. Is that enough to prevent bad solves? I think it is. You can't make a loop because you don't have enough trails. 

So in the initial scenario, you start with one object. Two of them appear out of the portal (here labeled 2).
When resolving the portal, one of them went to


Say the start and end are the same point S. Everything that existed before the first portal was created starts at S. Everything around after the portal dies ends at S. Now the question is: can you draw the whole graph without lfiting your pencil?

Anytime you resolve all the portals, it should check that that's the case.
If the graph is connected, can this ever fail to be the case?

No! Say you have a cycle. Since the graph is connected there's a node where the cycle abutts some other path. Break the cycle and splice it into the path. Repeat until there are no cycles.

Can you detect if the configuration is invalid "online", as soon as it becomes unsolvable?

Say in the intermediate state, you track each existing thing as an edge to `Escape`. When you create a portal, it will create a node for that portal, with an edge to `Escape`. The portal will be connected to the rest of the graph by the `Escape` edge.

When the portal is created, its input edges don't exist. You can't really be doomed until you fill those edges in, since until then you can always connect the graph back together by running something to the input edge. If you have a subgraph that isn't otherwise connected (that is to say, its only connection to the rest is through "escape"), no edges from the rest of the graph ave filled those edges in. This means that the number of edges to "escape" equals the number of open slots, so as long as there are edges to "escape" an otherwise disconnected graph can be connected. By the same argument, if a graph is completely disconnected it can't have any free openings, and is this doomed.

Therefore, a graph is not doomed iff it's connected, under the scheme where free edges are linked to "escape".

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



