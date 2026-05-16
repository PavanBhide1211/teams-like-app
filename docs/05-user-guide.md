# Cowork Chat — User Guide

> *Audience: anyone using Cowork Chat as a chat client. No technical background assumed.*

---

## What is Cowork Chat?

Cowork Chat is a lightweight desktop chat app for teams. It looks and behaves a lot like Microsoft Teams or Slack — workspaces, channels, direct messages, threads, reactions — but it is built to be lean: low memory, low CPU, fast to open, and quiet in the background.

It runs as a small native window on your laptop (Windows, macOS, or Linux) and a tray icon that stays out of your way. Everything happens over an encrypted connection to your Cowork server.

## What you will be doing in this guide

1. Install and launch the app.
2. Sign in or create your account.
3. Pick a workspace and join channels.
4. Send messages, reply in threads, react with emoji, mention teammates.
5. Use direct messages.
6. Change your presence (online / away / do-not-disturb).
7. Use the system tray.
8. Adjust notification preferences.
9. Sign out.
10. Troubleshoot common issues.

You can read top to bottom (10 minutes), or jump straight to the section you need.

---

## 1. Installing and launching

### Windows

1. Download `Cowork-Chat-Setup.exe` from your IT admin or the Cowork release page.
2. Double-click to run. Accept the security prompt.
3. The installer places Cowork Chat in your Start menu.
4. Launch from Start menu, or pin to taskbar.

### macOS

1. Download `Cowork-Chat.dmg`.
2. Double-click; drag **Cowork Chat** to the **Applications** folder.
3. Launch from Applications (or Spotlight: ⌘-Space → "Cowork Chat").
4. The first launch will ask for permission to send notifications — choose **Allow**.

### Linux

1. Download the `.deb` (Debian/Ubuntu) or `.AppImage` (any distro).
2. `.deb`: `sudo dpkg -i cowork-chat_0.1.0_amd64.deb`.
3. `.AppImage`: `chmod +x cowork-chat.AppImage`, then run it.
4. Launch from your applications menu.

> **Why so small?** The app is around 5–10 MB on disk and uses about 50–80 MB of RAM at idle — comparable to a single browser tab, far smaller than typical Electron apps. This is by design.

---

## 2. Signing in (or creating an account)

When you launch Cowork Chat for the first time you see the **Sign in** screen.

### If you already have an account

- Enter your **email** and **password**.
- Click **Sign in**.

### If you do not have an account

- Click **Create one** at the bottom.
- Enter:
  - **Display name** — what teammates see (e.g. "Pavan B.").
  - **Email** — must be valid; you will use it to sign in.
  - **Password** — at least **12 characters**. Long passphrases are stronger than short complex ones — `correct horse battery staple` is harder to crack than `Aa1!`.
- Click **Create account**.

You will be signed in automatically after registering.

> **Forgot your password?** The lean demo does not yet have a self-service reset flow. Ask your workspace admin to provision a new account, or set up password reset via your identity provider if your deployment includes one.

---

## 3. Workspaces and channels

A **workspace** is your organisation's space on Cowork. A **channel** is a topic-scoped conversation inside the workspace (e.g. #general, #engineering, #lunch). You can be a member of many workspaces; pick which one you are looking at from the dropdown at the top-left.

### Picking your active workspace

Click the workspace dropdown at the top of the left sidebar and choose the workspace you want.

### Joining a channel

- **Public channels** (marked with `#`) — any member of the workspace can read and post. They show up automatically in your sidebar.
- **Private channels** (marked with a padlock 🔒) — invitation only. You will only see them in the sidebar if you have been added by an existing member or a workspace admin.

To open a channel, click its name in the sidebar.

### Creating a channel

Right now in the lean demo, channels are created via the workspace admin. Ask your admin, or — if you are the admin — use the API:

```bash
curl -X POST http://your-cowork/workspaces/<workspace-id>/channels \
  -H "Authorization: Bearer <your-token>" \
  -H "Content-Type: application/json" \
  -d '{"name":"engineering","topic":"build chat","kind":"public"}'
```

A "create channel" UI is planned for a future release.

---

## 4. Reading and sending messages

### The message list

The main area is the message list for the channel or DM you have open.

- Messages are shown oldest at the top, newest at the bottom.
- Each message shows the sender, the time, the body, and any reactions.
- A small `(edited)` label appears next to a message that has been edited.
- Deleted messages disappear and leave a "this message was deleted" tombstone.

### Sending a message

- Click the composer at the bottom of the channel.
- Type your message.
- Press **Enter** to send. To go to a new line without sending, press **Shift+Enter**.
- The send button (paper-plane icon) sends as well.

### Formatting

Cowork Chat supports a small markdown subset:

| Syntax | Result |
|---|---|
| `**bold**` | **bold** |
| `*italic*` | *italic* |
| `` `code` `` | `code` |
| triple-backtick block | code block |
| `[link text](https://example.com)` | clickable link |

Plain newlines render as line breaks.

### Mentioning a teammate

Type `@` followed by a few letters of their name. A small popover lists matching members of the current channel. Click one (or press Enter) to insert the mention. Mentioned people get a notification.

### Replying in a thread

Hover over a message → click the **Reply** icon (or right-click → Reply). A thread panel opens on the right where you and the original author can have a focused sub-conversation. The parent channel sees a small "1 reply" indicator instead of every back-and-forth.

### Reacting with emoji

Hover over a message → click the small smiley → pick an emoji. Click an existing reaction to add yourself, click again to remove yourself.

### Editing your own message

Hover over your own message → **Edit**. Adjust the text, press **Enter** to save. The message will show `(edited)`.

### Deleting your own message

Hover over your own message → **Delete**. The message is replaced with a "this message was deleted" tombstone. The deletion is logged for audit purposes.

---

## 5. Direct messages (DMs)

Use DMs for 1:1 or small-group conversations that don't belong in a channel.

### Starting a DM

- In the sidebar, scroll to **Direct messages**.
- Click **New DM** (or use ⌘/Ctrl-K and start typing a person's name).
- Type the email(s) of the people to include.
- Click **Start**.

The DM thread is created and opens immediately. You can have a DM with one person (1:1) or several (group DM). The composer and behaviour are identical to a channel.

### Finding an existing DM

Existing DM threads appear in the sidebar under **Direct messages**. They are sorted with the most recent activity at the top.

---

## 6. Presence — letting people know if you're available

Your status indicator appears as a small dot next to your avatar. Teammates see it next to your name everywhere.

The available statuses:

| Status | Dot | Meaning |
|---|---|---|
| Online | 🟢 green | You're at the computer and reachable. |
| Away | 🟡 amber | You're around but not actively at the keyboard. |
| Do not disturb | 🔴 red | You don't want notifications. Mentions still arrive but go silent. |
| Offline | ⚪ grey | Your app is closed or you've lost network. Set automatically. |

### Changing your status

Click your avatar at the bottom of the sidebar → **Set status** → pick one. The change is broadcast immediately to everyone who shares a channel or DM with you.

You cannot manually set yourself to *Offline* — the system marks you offline ~30 seconds after your app loses its connection.

---

## 7. The system tray

When you close the main window, Cowork Chat keeps running in the system tray (Windows / Linux) or menu bar (macOS) so you keep receiving messages and notifications.

### What the tray icon does

- **Left-click** the tray icon → show the main window.
- **Right-click** (Windows / Linux) or click (macOS) → menu with:
  - **Show window** — bring the main window back.
  - **Quit Cowork Chat** — fully exit the app. Quitting stops notifications until you launch again.

If you actually want the app gone for a while, right-click → **Quit Cowork Chat**. Closing the window alone does not quit it.

---

## 8. Notifications

By default, the app sends a desktop notification when:

- Someone sends a message in a channel that mentions you (`@your-name`).
- Someone sends a message in a DM you're a member of.

It does **not** notify for every message in every channel — that would be unbearable. You can change this behaviour per channel.

### Per-channel notification settings

In the channel header, click the bell icon. Pick:

- **All messages** — notify on every message.
- **@mentions only** (default for channels) — notify only when you're mentioned.
- **Nothing** — never notify; the channel will not even show as unread.

### Quiet hours (do-not-disturb)

Set your status to **Do not disturb** (Section 6). While DND:
- Desktop notifications are suppressed.
- Sounds are suppressed.
- Mentions are still recorded; you'll see them when you return.

### macOS-specific note

On first launch, macOS asks if you want to allow notifications. If you accidentally declined, open **System Settings → Notifications → Cowork Chat** and turn them on.

---

## 9. Signing out

Click your avatar at the bottom of the sidebar → **Log out**. You will be returned to the sign-in screen.

Quitting the app (Section 7) does **not** sign you out — your session persists until you explicitly log out, or until your refresh token expires (typically 30 days).

---

## 10. Troubleshooting

### "I can't sign in"

- Double-check the email is exactly what you registered with. Email is case-insensitive but otherwise must match.
- Password must be at least 12 characters. If you set yours shorter previously, ask your admin to reset it.
- If you see "invalid credentials" repeatedly, you may have been rate-limited. Wait 5 minutes and try again.

### "I joined but I can't see any channels"

- Check the workspace dropdown at the top of the sidebar. You may be in a workspace that has no channels. Ask your admin to add you to channels, or to create the right channels in the workspace.

### "Messages are not arriving in real time"

- Look at the dot in the lower-right corner of the window. If it is grey/red, the WebSocket connection is down. The app reconnects automatically with backoff; usually fixes itself within a minute.
- If your laptop just woke from sleep, give it 5–10 seconds.
- If the problem persists, quit the app (tray → Quit) and relaunch.

### "I get duplicate notifications"

- You may have the app open on more than one machine. Quit the duplicates or sign out from them.

### "The app is using more memory than expected"

- A typical idle Cowork Chat session sits around 50–100 MB RAM. If you see substantially more:
  - Switch away from a channel with tens of thousands of messages; the virtualised list lets only a small window stay in memory, but switching out frees more.
  - Restart the app if it's been running for many days.
  - If the high usage persists, file a bug report (Section 11).

### "I deleted a message by mistake"

- Deletions are soft — they remain in the database for audit. Your workspace admin can ask the server administrator to undelete a specific message. The lean demo doesn't expose this in the UI yet; recovery is a SQL-side operation by an admin.

### "I see a 'connection refused' or 'unable to reach server' error"

- Your laptop's network may be offline (check by loading any web page in your browser).
- Your VPN may be disconnected. Connect and retry.
- Cowork backend may be down. Ask your admin to check the server status.

---

## 11. Reporting a bug

If something doesn't work as described in this guide:

1. **Reproduce it once more**. Note the exact steps.
2. **Take a screenshot** if visual.
3. **Note your OS, the Cowork Chat version** (Help → About → Version), and what time it happened.
4. Send those four things to your admin or to `bugs@your-cowork-org.example`.

Please don't include passwords or copies of private messages unless they are essential to reproducing the problem — bug reports are read by your IT team.

---

## 12. Keyboard shortcuts

| Action | Shortcut (Win/Linux) | Shortcut (macOS) |
|---|---|---|
| Open quick-switcher | Ctrl-K | ⌘-K |
| Send message | Enter | Enter |
| New line in composer | Shift+Enter | Shift+Enter |
| Cycle channels | Alt-↑ / Alt-↓ | ⌥-↑ / ⌥-↓ |
| Mark all as read | Shift-Esc | Shift-Esc |
| Toggle DND | Ctrl-Shift-D | ⌘-Shift-D |
| Show / hide sidebar | Ctrl-\ | ⌘-\ |
| Settings | Ctrl-, | ⌘-, |

---

## 13. Privacy and data

A few facts worth knowing:

- **Your messages are stored on your organisation's Cowork server**, not on a third-party cloud. The deployment your admin runs is the only place messages live.
- **Passwords are never stored in plain text**. Cowork uses argon2id with strong parameters; even if the database were leaked, cracking your password would be extremely expensive.
- **Mentions, channel membership, and reactions are stored alongside the messages.** Deleting a message hides it from the UI; the row remains in the database for audit and is permanently removed by a retention sweep your admin configures.
- **Presence is ephemeral** — only your current status (online/away/dnd) and a timestamp are stored. Once you sign out and the time-to-live expires (~45 seconds), even that goes away.
- **The app does not phone home**. Cowork Chat talks only to the Cowork backend your admin pointed it at. It does not contact any analytics service, ad network, or third-party CDN.

If you have privacy questions specific to your deployment (retention, GDPR data subject rights, etc.), please ask your admin or DPO.

---

## 14. Glossary

- **Workspace** — Top-level container. One per organisation.
- **Channel** — Topic-scoped conversation inside a workspace.
- **Public channel** — Any workspace member can read and post.
- **Private channel** — Invitation-only.
- **DM** — Direct message; 1:1 or small group.
- **Thread** — A sub-conversation under a single message.
- **Mention** — `@name` reference that pings the user.
- **Presence** — Your availability status (online/away/dnd/offline).
- **Tray** — The icon area at the bottom-right of the Windows taskbar, the top-right of the macOS menu bar, or wherever your Linux desktop puts system-tray icons.

---

## 15. Where to get help

- This guide: re-read the relevant section.
- Your workspace admin: knows about workspace setup, channels, accounts.
- Your IT team: knows about the server, networking, and identity.
- Anthropic-style "ask the bot" channels: not built into the lean demo. A future release may add an `/ask` helper.

Have fun. Be kind. Stay on topic — or invent a new #lunch channel for the off-topic chat.
