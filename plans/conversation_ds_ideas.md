This is pretty good, I think we need to rethink the data model. We need to be able to restore a session from any checkpoint, which means we will have a branching message history as soon as we restore and continue. This means that the session is really not the right abstraction layer.  Instead we just need to build what is essentially a doubly linked list of message objects by id.  Then we can construct a session from the linked messages. One one hand, it takes more database queries, but those will be ultra fast.  We will need to construct the message history whenever the user wants to navigate the session.   I think the main navigation methods will be exploring the leaves of the sessions by the leaf's timestamp, then constructing the history of the leaf for the user to work with. 

There is also an edit mode I want to keep track of. This means actually modifying the history, including the LLM response of a message. However, this would not actually edit our database messages, instead, it would create a branch in the history at the point of the edit... but, this could introduce branching management bidirectionally, e.g. if we were to edit the first response in a 20 message exchange, we would essentially need to have the new history item also point to its parent and child nodes. This means we would have multiple references on each node.  On the other hand, we could have some system that identifies peer messages in a single data object, but only some of the peers would have child messages, so i think that is worse.  I think the simplest from data persepctive is multiple parents and children, but it creates a challenge when constructing the history. 

Let's think from the user message, the user would probably want to have a distinct "conversation" object, and if they have a divergent history, it would be viewable as a new "conversation".   We can construct these conversations on the fly but with multiple parents and children, they may become combinatorial.

---
*2025-09-14*

It seems like we only need singly linked, where the parentless nodes are the roots, and all nodes are the leaves, however, then we are not able to easily understand which messages start the conversation. We essentially can construct a conversation per leaf, plus a conversation for each multiple parent in any conversation tree. it helps if we invert it, so the latest message is the root, and the first message in a conversation is the leaves...

---
*2025-09-14*

Let's think through what will happen when we try to review the mesage history. We will have a list of conversations. Each will have a title and a timestamp.  This list needs to be constructed, which requires numerous sql calls per conversation. We actually never need to branch on a multiple parent, because two conversations do not converge to a common "latest message", they only diverge from a common initial message.  We might like to use the message hash and the parent hashes to identify a message as unique, then a later message that happens to use the same root could make that link apparent to the user through some ui or data exploration.

So, our logic would be, to insert a new message, we search for its conversation history hash, which is all messages before it hashed. Effectively, we could traverse the history, starting at the root, we hash the root message, then we hash the first descendant and concatenate its parent hashes (must be ordered though), and we continue to the leaf.  That would never actually happen though, since message history is accretive, we just take the message's parent hash and append it to the message, then we search if that message was already asked. If not, we store the message without a parent hash (new root). 

Oh, but we also have to consider some more parameters. We also have a response per message. If we don't have a response we can still store the message, but it's not "answered" whatever that means. I guess any message could be queued for response.  When queueing a message for response you would use some model with lots of parameters, a few are model name, quantization, temperature, provider/endpoint, and timestamp, and more.  This really is only relevant to the response messages. This adds another layer of complexity that can grow over time, for example, we might have a new parameter arise that needs to be taken into account.

We should really consider how far we want to go with storing the conversation history data. I think the ideal thing is just to have a metadata column that is sqlite text/json data type, which we will store as json for now.  Then we can extend the metadata over time, and if necessary we can reprocess old responses, but for the most part, they will just not have the new parameters that might get added.

The user message metadata would just be like timestamp I guess, and we could store how the message was submitted.

One common situation might be to generate responses in triplicate, so we would have 3 different leaf llm messages from a common root user message.

So, to generate the view of that would be a pain, because now we need to generate 3 conversations per message, so a round of 20 messages may have 3^20 conversations, we don't want to generate 3.3 billion conversation objects per chat. Although, actually the user would pick a winner, so it would be more like 2*n+3, where the 2 indicates the 2 non-selected messages, at 20 it would just be 43 conversations -- so nevermind. That's not bad. But we would still want to somehow prune the non-taken paths, I am thinking about something like a conversation tree TUI view, where you have to press spacebar to expand each parent of the leaf. How would we know which one the user selected?  We could expose the leaf node with the latest timestamp, then the user could traverse backwards, expanding, and also they could use search from the root and tag useful nodes.

This problem is well beyond our current scope. As long as we have the data structure right, we will revisit the presentation later.

My decision point is between a doubly linked list or a singly linked list traversing from leaf to root, and whether the leaf is the last message or the root.

To make things easier, let's have the root be the parentless message, then the leaf will be the latest message in the branch.  Then, it's very easy to visualize.  We will have a method to get the message root, then we have a method to get common leaves, which traverses backwards to the root, then traverses forwards and identifies all the leaf nodes. This requires bidirectional traversal. To implement with single direction traversal, from root to leaf, we must first find the root of the leaf, which means we search every single root for that leaf, then find all its other leaves.  To single direction from leaf to root, we must traverse from leaf to root, then traverse every other leaf to root, then filter in the common root.  An alternative is maintaining a leaf-root index, so we know which leaves belong to which root.  With this index, we only need single direction, either leaf to root or root to leaf are sufficient.

---
*2025-09-14*

I still have the issue of editing the parent message. I guess I can defer this and simply not deal with it, but I want the data structure to support it if I add the feature. The user may want to edit the LLM message to inject a behavior.  Also, the user may want to edit their own message, e.g. for the same reason. Adding new messages faces a problem like this, where it essentially adds a multi key child, but then the child of that message converges back to the original tree structure.  Both introduce a situation where the descendant messages have multiple parents.

However, in the message uniqueness model, an edited message requires rehashing all the child model parents, so maybe it duplicates all the child messages forming a new descendant tree and new leaf node.
