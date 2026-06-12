#!/usr/bin/env python3
"""
AINMS Agent Creator Bot
A simple, robust Playwright bot that logs into the AINMS admin dashboard,
creates a new agent with auto-generated Lorem Ipsum data, and verifies its creation.
"""

import os
import sys
import time
from dataclasses import dataclass
from typing import Optional

from dotenv import load_dotenv
from playwright.sync_api import (
    Page,
    Playwright,
    sync_playwright,
    TimeoutError as PlaywrightTimeout,
)
from rich.console import Console
from rich.panel import Panel
from rich.text import Text

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
load_dotenv()

BASE_URL: str = os.getenv("AINMS_BASE_URL", "http://localhost:3440")
ADMIN_EMAIL: str = os.getenv("AINMS_ADMIN_EMAIL", "admin@ainms.local")
ADMIN_PASSWORD: str = os.getenv("AINMS_ADMIN_PASSWORD", "admin123")
HEADLESS: bool = os.getenv("AINMS_HEADLESS", "false").lower() == "true"
VIDEO_DIR: str = os.getenv("AINMS_VIDEO_DIR", "./videos")

# ---------------------------------------------------------------------------
# Simple Lorem Ipsum Generator (zero-dependency)
# ---------------------------------------------------------------------------
LOREM_WORDS = [
    "lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit",
    "sed", "do", "eiusmod", "tempor", "incididunt", "ut", "labore", "et", "dolore",
    "magna", "aliqua", "ut", "enim", "ad", "minim", "veniam", "quis", "nostrud",
    "exercitation", "ullamco", "laboris", "nisi", "ut", "aliquip", "ex", "ea",
    "commodo", "consequat", "duis", "aute", "irure", "dolor", "in", "reprehenderit",
    "in", "voluptate", "velit", "esse", "cillum", "dolore", "eu", "fugiat", "nulla",
    "pariatur", "excepteur", "sint", "occaecat", "cupidatat", "non", "proident",
    "sunt", "in", "culpa", "qui", "officia", "deserunt", "mollit", "anim", "id",
    "est", "laborum",
]


def lorem_words(count: int) -> str:
    """Generate a string of `count` random Lorem Ipsum words."""
    from random import choices
    return " ".join(choices(LOREM_WORDS, k=count)).capitalize() + "."


def lorem_name() -> str:
    """Generate a plausible agent name."""
    from random import choice
    adjectives = ["Alpha", "Beta", "Gamma", "Delta", "Omega", "Prime", "Core", "Lite"]
    nouns = ["Sentinel", "Watcher", "Bot", "Agent", "Node", "Drone", "Sentry", "Guard"]
    return f"{choice(adjectives)} {choice(nouns)}"


# ---------------------------------------------------------------------------
# Logging Helper
# ---------------------------------------------------------------------------
console = Console()


def log_step(step: int, total: int, message: str, detail: Optional[str] = None) -> None:
    """Pretty-print a bot step."""
    header = Text(f"[{step}/{total}]", style="bold magenta")
    msg = Text(message, style="bold white")
    if detail:
        msg.append(Text(f" ({detail})", style="dim"))
    console.print(Panel(msg, title=header, border_style="green", width=80))


def log_error(message: str) -> None:
    console.print(Panel(Text(message, style="bold red"), border_style="red", title="ERROR"))


def log_success(message: str) -> None:
    console.print(Panel(Text(message, style="bold green"), border_style="green", title="SUCCESS"))


# ---------------------------------------------------------------------------
# Bot Core
# ---------------------------------------------------------------------------
@dataclass
class AgentData:
    name: str
    description: str


class AinmsBot:
    """Playwright bot for the AINMS admin dashboard."""

    def __init__(self, playwright: Playwright) -> None:
        self.playwright = playwright
        self.browser = playwright.chromium.launch(headless=HEADLESS)
        self.context = self.browser.new_context(
            record_video_dir=VIDEO_DIR if not HEADLESS else None,
            viewport={"width": 1280, "height": 720},
        )
        self.page: Page = self.context.new_page()
        self.page.set_default_timeout(15_000)

    # --- Navigation Helpers ---
    def _goto(self, path: str) -> None:
        url = f"{BASE_URL.rstrip('/')}{path}"
        log_step(0, 0, "Navigating", url)
        self.page.goto(url)

    def _click(self, selector: str, description: str) -> None:
        log_step(0, 0, f"Clicking", description)
        self.page.click(selector)

    def _fill(self, selector: str, value: str, description: str) -> None:
        log_step(0, 0, f"Filling", description)
        self.page.fill(selector, value)

    def _expect_visible(self, selector: str, description: str) -> None:
        log_step(0, 0, "Waiting for", description)
        self.page.wait_for_selector(selector, state="visible")

    # --- Workflow Steps ---
    def login(self) -> None:
        """Log into the admin dashboard."""
        log_step(1, 6, "Logging in", f"as {ADMIN_EMAIL}")
        self._goto("/login")
        self._fill('input[name="email"]', ADMIN_EMAIL, "Email field")
        self._fill('input[name="password"]', ADMIN_PASSWORD, "Password field")
        self._click('button[type="submit"]', "Login button")
        # Wait for redirect to dashboard (URL should no longer be /login)
        self.page.wait_for_url(lambda url: "/login" not in url, timeout=10_000)
        log_success("Login successful!")

    def navigate_to_agents(self) -> None:
        """Click the 'Agents' link in the sidebar."""
        log_step(2, 6, "Navigating to", "Agents page")
        # Use text-based selector for resilience against Tailwind class changes
        self._click('a:has-text("Agents")', "Agents sidebar link")
        self._expect_visible('h1:has-text("Agents")', "Agents heading")
        log_success("On Agents page.")

    def create_agent(self) -> AgentData:
        """Click 'Add New Agent', fill the form, and save."""
        log_step(3, 6, "Creating new agent")
        self._click('button:has-text("Add New Agent")', "Add New Agent button")
        self._expect_visible('h2:has-text("Add New Agent")', "Add Agent modal/page")

        data = AgentData(name=lorem_name(), description=lorem_words(12))
        self._fill('input[name="name"]', data.name, "Agent Name")
        self._fill('textarea[name="description"]', data.description, "Agent Description")
        self._click('button:has-text("Save")', "Save button")

        # Wait for success toast or redirect back to list
        try:
            self.page.wait_for_selector('text=Agent created', timeout=5_000)
        except PlaywrightTimeout:
            # Fallback: wait for URL to return to /agents or the list to reappear
            self.page.wait_for_selector('h1:has-text("Agents")', timeout=5_000)

        log_success(f"Agent '{data.name}' created successfully.")
        return data

    def verify_agent(self, expected_name: str) -> None:
        """Navigate back to Agents and assert the agent exists."""
        log_step(4, 6, "Verifying agent", expected_name)
        self.navigate_to_agents()
        # Wait for the table/card list to contain our agent name
        self._expect_visible(f'text={expected_name}', f"Agent '{expected_name}' in list")
        log_success(f"Verified: '{expected_name}' is present in the Agents list.")

    def run(self) -> None:
        """Execute the full workflow."""
        total_steps = 5
        try:
            self.login()
            self.navigate_to_agents()
            agent = self.create_agent()
            self.verify_agent(agent.name)
            log_step(5, total_steps, "Workflow complete", "All checks passed.")
            log_success("Bot finished successfully. A new agent was created and verified.")
        except PlaywrightTimeout as e:
            log_error(f"Timeout waiting for an element: {e}")
            self._take_screenshot("error_timeout.png")
            sys.exit(1)
        except Exception as e:
            log_error(f"Unexpected error: {e}")
            self._take_screenshot("error_unknown.png")
            sys.exit(1)
        finally:
            self.close()

    def _take_screenshot(self, filename: str) -> None:
        path = os.path.join(VIDEO_DIR, filename)
        self.page.screenshot(path=path, full_page=True)
        console.print(f"[yellow]Screenshot saved to {path}[/yellow]")

    def close(self) -> None:
        log_step(0, 0, "Cleaning up", "Closing browser")
        self.context.close()
        self.browser.close()


# ---------------------------------------------------------------------------
# Entry Point
# ---------------------------------------------------------------------------
def main() -> None:
    console.rule("[bold blue]AINMS Agent Creator Bot[/bold blue]")
    console.print(f"Base URL: {BASE_URL}")
    console.print(f"Headless: {HEADLESS}")
    console.print(f"Video Dir: {VIDEO_DIR}")
    console.rule()

    os.makedirs(VIDEO_DIR, exist_ok=True)

    with sync_playwright() as p:
        bot = AinmsBot(p)
        bot.run()


if __name__ == "__main__":
    main()
