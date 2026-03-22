# 23 — Web Omniscience

## Hydra Knows Where to Find Anything

Hydra does not store the internet. It indexes it. And over time, it internalizes it. The difference between a search engine and omniscience is that a search engine finds answers. Hydra remembers them forever and never searches for the same thing twice.

---

## The Three Layers

### Layer 1: Genome (Zero Web Calls)

If the answer is already in the genome — from a previous lookup, from a skill file, or from a self-written entry — Hydra answers instantly. No internet required.

```
You: "What is the circuit breaker pattern?"
Hydra: [genome hit, conf=92%, obs=5000]
       "Install a circuit breaker at every external dependency boundary..."

Web calls made: 0
Response time: milliseconds
```

This is where 90% of your questions end up after month 6. The genome grows from every interaction. The internet becomes internalized.

### Layer 2: Index (One Targeted Call)

If the genome does not have the answer but Hydra knows WHERE to find it, one targeted call goes directly to the right source. No searching. No browsing. Direct hit.

```
You: "How does Kubernetes pod scheduling work?"
Hydra: [index hit: kubernetes.io/docs/ reliability=97%]
       → One call to kubernetes.io
       → Extracts the answer
       → Stores in genome for next time

Web calls made: 1 (targeted)
Next time this question is asked: 0 calls (Layer 1)
```

The knowledge index ships with 21 foundational sources covering programming languages, frameworks, architecture patterns, science, finance, and AI/ML. Every successful lookup adds more sources.

### Layer 3: Search (One Broad Call, Then Internalize)

If the topic is unknown — not in the genome, not in the index — Hydra searches the web. But it only searches once per topic. The result is indexed and crystallized.

```
You: "What is quantum teleportation?"
Hydra: [not in genome, not in index]
       → One search call (Brave/Google)
       → Finds best source (Wikipedia, 90% reliability)
       → Indexes the source for next time
       → Crystallizes the answer as a genome entry

Web calls made: 1 (search)
Next time: 0 calls (genome has it)
The source is indexed: future quantum physics questions go directly to it
```

---

## The Cycle of Internalization

```
Day 1:     You ask 20 questions. Hydra makes 15 web calls.
Week 1:    You ask 20 questions. Hydra makes 8 web calls.
           (7 answered from genome — topics you revisited)
Month 1:   You ask 20 questions. Hydra makes 4 web calls.
           (16 answered from genome — your domain is forming)
Month 6:   You ask 20 questions. Hydra makes 1 web call.
           (19 answered from genome — your domain is internalized)
Year 1:    You ask 20 questions. Hydra makes 0 web calls most days.
           (The internet is inside the genome for your domain)
```

This is not caching. Caching stores responses. Hydra stores understanding. The genome entry for "circuit breaker" is not a cached web page — it is a proven approach with confidence scores, observation counts, and Bayesian updating.

---

## The Dream Loop Explores

While you sleep, Hydra does not just consolidate memories. It explores.

```
Today you asked about gRPC.
Tonight the dream loop:
  1. Notices "gRPC" appeared 3 times in today's exchanges
  2. Checks the index: no source for gRPC
  3. Queues for exploration: grpc.io should be indexed
  4. When the daemon runs exploration:
     → Fetches grpc.io documentation summary
     → Indexes it as a source (reliability=96%)
     → Crystallizes key concepts as genome entries

Tomorrow:
  You: "How does gRPC handle streaming?"
  Hydra: [genome hit — crystallized overnight]
  Web calls: 0
```

The daemon must be running for this. The dream loop is where exploration happens.

---

## Seeded Knowledge Sources

Hydra ships with 21 foundational sources. These are the "shelves" the librarian already knows:

```
PROGRAMMING:
  Rust        → doc.rust-lang.org
  Python      → docs.python.org
  JavaScript  → developer.mozilla.org
  TypeScript  → typescriptlang.org
  Go          → go.dev/doc

INFRASTRUCTURE:
  Kubernetes  → kubernetes.io/docs
  Docker      → docs.docker.com
  Terraform   → developer.hashicorp.com/terraform

ARCHITECTURE:
  Circuit Breaker  → martinfowler.com
  Microservices    → microservices.io
  Design Patterns  → refactoring.guru

SCIENCE:
  Physics     → Wikipedia
  Chemistry   → Wikipedia
  Biology     → Wikipedia
  Mathematics → Wikipedia

FINANCE:
  Markets     → Investopedia
  Crypto      → Wikipedia

AI/ML:
  Machine Learning → scikit-learn.org
  Neural Networks  → Wikipedia
  Transformers     → huggingface.co
```

These grow automatically as Hydra encounters new topics.

---

## The Integrations

Three web integrations ship ready to use:

### Brave Search — Search Anything
```
integrations/web-search/api.toml
  search: GET /web/search?q={query}&count=5
  news:   GET /news/search?q={query}&count=5
```

### GitHub Knowledge — Read the World's Code
```
integrations/github-knowledge/api.toml
  search-repos:  find repositories by topic
  search-code:   search code across all public repos
  readme:        read any repository's README
  file-content:  read any file in any public repo
  issues:        read issues and discussions
  commits:       read commit history
```

### Wikipedia — The World's Encyclopedia (No API Key)
```
integrations/wikipedia/api.toml
  summary:       get summary of any article
  full-article:  get full content
  search:        search for any topic
```

---

## How It Connects to Everything Else

```
Web Search → finds answer → GENOME (crystallizes as permanent entry)
                           → INDEX (source URL remembered)
                           → MEMORY (exchange stored in .amem)
                           → CALIBRATION (tracks accuracy)
                           → SELF-WRITING (automation detects patterns)

GitHub → reads code → GENOME (approach extracted)
                     → CARTOGRAPHY (system mapped)
                     → SKILLS (can generate new skill from repo)

Wikipedia → reads article → GENOME (facts stored)
                           → BELIEFS (updated from authoritative source)
                           → SYNTHESIS (cross-domain connections)
```

---

## The Social Media Version

```
ChatGPT searches the web every time you ask.
Hydra searches once. Remembers forever.

ChatGPT does not know WHERE to look.
Hydra has an index of 21+ sources — goes directly to the right one.

ChatGPT forgets what it found next conversation.
Hydra crystallizes every finding into permanent genome entries.

Day 1: both make the same number of web calls.
Month 6: ChatGPT still makes the same number.
         Hydra makes almost none.

The internet becomes part of Hydra.
Not cached. Internalized. Permanent. Growing.
```
