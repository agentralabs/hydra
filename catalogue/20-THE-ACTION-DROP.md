# 20 — The Action Drop

## Teach Hydra to DO Things. No Code.

Skills make Hydra knowledgeable. Integrations make Hydra connected. Actions make Hydra **capable**. An action is something Hydra executes — a shell command, an API call, a scheduled task.

Create a folder. Drop an `action.toml`. Hydra can now do that thing.

---

## What an Action Looks Like

```
actions/
  deploy/
    action.toml     ← what to execute, when, with what approval
```

One file. One folder. Hydra picks it up on boot.

---

## action.toml — Telling Hydra What to Do

### Manual Action (you ask, Hydra does)

```toml
[action]
name        = "deploy"
description = "Deploy the application to production"
trigger     = "manual"
approval    = "required"

[action.execute]
type            = "shell"
command         = '''
cd /path/to/project && \
git pull origin main && \
docker-compose build && \
docker-compose up -d
'''
timeout_seconds = 300

[[action.parameters]]
name     = "environment"
type     = "string"
default  = "production"
```

```
You: "Deploy to production"
Hydra: "This will pull latest code, build containers, and deploy.
        Environment: production. Timeout: 5 minutes. Approve?"
You: "yes"
Hydra: [executes the command]
       "Deployment complete. Exit code 0. Duration: 47 seconds.
        Receipt: act-2026-03-21-001."
```

### Scheduled Action (runs automatically)

```toml
[action]
name        = "backup-database"
description = "Backup the production database to S3"
trigger     = "scheduled"
schedule    = "every day at 3:00 AM"
approval    = "notify"

[action.execute]
type            = "shell"
command         = '''
pg_dump $DATABASE_URL | gzip | \
aws s3 cp - s3://backups/db-$(date +%Y%m%d).sql.gz
'''
timeout_seconds = 600
```

```
[No interaction needed]
3:00 AM — Hydra runs the backup
3:02 AM — Backup complete
Morning — Briefing: "○ Database backup completed at 3:02 AM (2.3 GB)"
```

### Conditional Action (triggers on events)

```toml
[action]
name        = "alert-high-cpu"
description = "Alert when CPU usage exceeds 90%"
trigger     = "conditional"
approval    = "auto"

[action.execute]
type            = "shell"
command         = 'osascript -e "display notification \"CPU at {cpu_pct}%\" with title \"Hydra Alert\""'
timeout_seconds = 5

[action.condition]
check_command   = "top -l 1 | grep 'CPU usage' | awk '{print $3}' | tr -d '%'"
threshold       = 90
comparison      = "greater_than"
check_interval  = "every 5 minutes"
```

### API Action (calls an external API)

```toml
[action]
name        = "create-jira-ticket"
description = "Create a Jira ticket from conversation context"
trigger     = "manual"
approval    = "required"

[action.execute]
type    = "api"
method  = "POST"
url     = "https://your-company.atlassian.net/rest/api/3/issue"
headers = { "Content-Type" = "application/json" }
body    = '''
{
  "fields": {
    "project": {"key": "{project}"},
    "summary": "{title}",
    "description": "{description}",
    "issuetype": {"name": "Task"}
  }
}
'''
timeout_seconds = 30

[[action.parameters]]
name     = "project"
type     = "string"
required = true

[[action.parameters]]
name     = "title"
type     = "string"
required = true

[[action.parameters]]
name     = "description"
type     = "string"
required = true
```

```
You: "Create a Jira ticket for the auth bug we discussed"
Hydra: [fills parameters from conversation context]
       "Create ticket in PROJECT-X:
        Title: 'Auth token refresh fails after rotation'
        Description: [extracted from conversation]
        Approve?"
You: "yes"
Hydra: "Ticket PROJECT-X-1234 created. Receipt: act-2026-03-21-002."
```

---

## The Three Trigger Types

### Manual — You Ask

```
trigger = "manual"

You say it. Hydra does it.
"Deploy to staging"
"Generate the report"
"Send the update to Slack"
```

### Scheduled — It Runs on Time

```
trigger  = "scheduled"
schedule = "every 30 minutes"
schedule = "every day at 3:00 AM"
schedule = "every monday at 9:00 AM"
schedule = "every 1st of month"

Hydra's scheduler fires the action at the specified time.
No human needed. Results appear in the morning briefing.
```

### Conditional — It Runs on Events

```
trigger = "conditional"

Hydra monitors a condition and fires when it is true.
"When disk usage exceeds 80%"
"When build fails"
"When error rate exceeds 5%"
"When portfolio drops more than 3% in a day"
```

---

## The Three Approval Modes

```
approval = "required"
  Hydra asks: "Approve?" You must say yes.
  For: spending money, sending messages, modifying systems

approval = "auto"
  Hydra executes without asking.
  For: safe read-only actions, notifications, internal tasks

approval = "notify"
  Hydra executes and tells you it did.
  For: scheduled tasks, monitoring, backups
  "I backed up the database at 3:02 AM."
```

---

## Real-World Action Examples

### Generate a Video (Remotion)

```toml
[action]
name        = "generate-video"
description = "Generate a video using Remotion"
trigger     = "manual"
approval    = "required"

[action.execute]
type            = "shell"
command         = '''
cd /path/to/remotion-project && \
npx remotion render src/index.tsx \
  --props='{"title": "{title}", "content": "{content}"}' \
  out/{filename}.mp4
'''
timeout_seconds = 600

[[action.parameters]]
name     = "title"
type     = "string"
required = true

[[action.parameters]]
name     = "content"
type     = "string"
required = true

[[action.parameters]]
name     = "filename"
type     = "string"
default  = "output"
```

### Post to Social Media

```toml
[action]
name        = "post-twitter"
description = "Post a tweet"
trigger     = "manual"
approval    = "required"

[action.execute]
type    = "api"
method  = "POST"
url     = "https://api.twitter.com/2/tweets"
body    = '{"text": "{tweet_text}"}'
timeout_seconds = 15

[[action.parameters]]
name     = "tweet_text"
type     = "string"
required = true
```

### Run Tests Before Deploy

```toml
[action]
name        = "test-and-deploy"
description = "Run tests, then deploy if they pass"
trigger     = "manual"
approval    = "required"

[action.execute]
type            = "shell"
command         = '''
echo "Running tests..."
cargo test 2>&1
if [ $? -eq 0 ]; then
  echo "Tests passed. Deploying..."
  docker-compose up -d --build
  echo "Deployed successfully."
else
  echo "Tests FAILED. Deployment blocked."
  exit 1
fi
'''
timeout_seconds = 600
```

### Morning Standup Report

```toml
[action]
name        = "standup-report"
description = "Generate daily standup from yesterday's activity"
trigger     = "scheduled"
schedule    = "every day at 8:30 AM"
approval    = "notify"

[action.execute]
type            = "shell"
command         = '''
echo "=== STANDUP $(date +%Y-%m-%d) ==="
echo "Commits yesterday:"
cd /path/to/project && git log --since="yesterday" --oneline
echo ""
echo "Open PRs:"
gh pr list --state open --limit 5
echo ""
echo "Failed checks:"
gh run list --status failure --limit 3
'''
timeout_seconds = 30
```

---

## How Actions and Integrations Work Together

```
Integration = the connection (WHERE to reach)
Action      = the capability (WHAT to do)

Together:
  Integration provides the API connection.
  Action defines when and how to use it.

Example:
  integrations/github/api.toml     → connection to GitHub API
  actions/check-build/action.toml  → scheduled: check build every 30 min

  The action uses the integration's endpoint.
  The integration provides the authentication.
  The action defines the trigger and approval.
```

---

## Constitutional Enforcement

Every action, regardless of trigger or approval mode:

1. **Checked** against the 7 constitutional laws before execution
2. **Receipted** with SHA256 hash after execution
3. **Settled** in the cost ledger
4. **Stored** in memory for future reference

```
No action runs in the dark.
No action runs without a receipt.
No action runs that violates the constitution.
Even auto-approved scheduled tasks at 3 AM are fully audited.
```

---

## How to Create Your Own

```bash
# Step 1: Create the folder
mkdir actions/your-action

# Step 2: Write the action.toml
cat > actions/your-action/action.toml << 'EOF'
[action]
name        = "your-action"
description = "What this action does"
trigger     = "manual"
approval    = "required"

[action.execute]
type            = "shell"
command         = "echo 'Hello from Hydra'"
timeout_seconds = 10

[[action.parameters]]
name     = "message"
type     = "string"
required = true
EOF

# Step 3: Restart Hydra
# That is it. Hydra can now execute your action.
```

---

## The Complete Picture

```
skills/          → Hydra KNOWS        (genome + functor)
integrations/    → Hydra CONNECTS     (api + credentials)
actions/         → Hydra DOES         (execute + approve)

Combined:
  You: "The build failed. Create a Jira ticket and notify the team."

  Hydra:
    [skill]       Finance genome says: "first loss is the best loss"
                  (wrong domain — but shows genome fires for everything)
    [skill]       Coding genome says: "test behavior not implementation"
    [integration] Reads GitHub API for build failure details
    [action]      Creates Jira ticket with failure context
    [action]      Sends Slack notification to #engineering
    [receipt]     All three actions receipted
    [memory]      Stored for future reference

  Three TOML files made this possible.
  No code was written.
```

---

## The Social Media Version

```
To make ChatGPT run a command, you paste instructions every time.
To make Hydra run a command, you write it once in action.toml.

ChatGPT forgets the command next conversation.
Hydra runs it forever.

ChatGPT cannot schedule tasks.
Hydra backs up your database at 3 AM every night.

ChatGPT cannot monitor conditions.
Hydra alerts you when CPU hits 90%.

One TOML file. Any command. Any schedule. Any trigger.
Receipted. Audited. Constitutional.
Drop and go.
```
