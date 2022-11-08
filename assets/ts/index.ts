type Chore = {
  title: string;
  description: string;
  expected_completion_time: number;
  status: string;
};

type ListChoresResponse = {
  success: boolean;
  error?: string;
  chores: Chore[];
};

const fetchChores = async (): Promise<Chore[]> => {
  let response = await fetch("/api/chores");
  return (await response.json()).chores;
}

const createChoreCard = (chore: Chore): Node => {
  let cell = document.createElement("div");
  cell.classList.add("cell");
  cell.classList.add("large-auto");

  let card = document.createElement("div");
  card.classList.add("card");

  let cardDivider = document.createElement("div");
  cardDivider.classList.add("card-divider");

  cardDivider.classList.add("callout");
  if (chore.status === "completed") {
    cardDivider.classList.add("success");
  } else if (chore.status === "upcoming") {
    cardDivider.classList.add("secondary");
  } else if (chore.status === "missed") {
    cardDivider.classList.add("alert");
  } else if (chore.status === "overdue") {
    cardDivider.classList.add("warning");
  } else {
    cardDivider.classList.add("primary");
  }

  let title = document.createElement("h2");
  if (chore.status === "completed" || chore.status === "missed") {
    let struckOut = document.createElement("s");
    struckOut.textContent = chore.title;
    title.appendChild(struckOut);
  } else {
    title.textContent = chore.title;
  }

  cardDivider.appendChild(title);
  card.appendChild(cardDivider);

  let cardContent = document.createElement("div");
  cardContent.classList.add("card-section");

  let choreStatusText = "Status: " + chore.status;

  let choreStatus = document.createElement("p");
  choreStatus.textContent = choreStatusText;

  cardContent.appendChild(choreStatus);

  let expectedDate = new Date(chore.expected_completion_time * 1000);
  let expectedTime = document.createElement("p");
  let expectedTimeBold = document.createElement("strong");
  expectedTimeBold.textContent = "Due date: " + expectedDate.toLocaleString();
  expectedTime.appendChild(expectedTimeBold);
  cardContent.appendChild(expectedTime);

  let description = document.createElement("p");
  description.textContent = chore.description;
  cardContent.appendChild(description);

  if (chore.status === "assigned") {
    let completeButton = document.createElement("button");
    completeButton.type = "button";
    completeButton.classList.add("button");
    completeButton.classList.add("success");
    completeButton.classList.add("expanded");
    completeButton.classList.add("large");
    completeButton.textContent = "Mark Completed";

    completeButton.onclick = async (): Promise<void> => {
      const data = new URLSearchParams();
      data.append("title", chore.title);
      data.append("expected_completion_time", chore.expected_completion_time.toString());

      await fetch("/api/chores/complete", {
        method: "POST",
        body: data,
      });

      await setChores();
    };

    cardContent.appendChild(completeButton);
  }

  card.appendChild(cardContent);

  cell.appendChild(card);

  return cell;
}

const removeAllChildren = (parent: Node): void => {
    while (parent.firstChild) {
        parent.removeChild(parent.firstChild);
    }
}

const CHORE_FINAL_STATES = ["completed", "missed"];

const setChores = async (): Promise<void> => {
  let chores = await fetchChores();

  let choresNode = document.querySelector("#chores");
  if (choresNode == null) {
    return;
  }

  chores.sort((a: Chore, b: Chore): number => {
    if (CHORE_FINAL_STATES.includes(a.status) && b.status === "assigned") {
      return 1;
    }

    if (CHORE_FINAL_STATES.includes(b.status) && a.status === "assigned") {
      return -1;
    }

    return a.expected_completion_time - b.expected_completion_time;
  });

  removeAllChildren(choresNode);

  let counts: Map<string, number> = new Map([
    ["assigned", 0],
    ["upcoming", 0],
    ["overdue", 0],
    ["missed", 0],
    ["completed", 0],
  ]);

  for (let chore of chores) {
    choresNode.appendChild(createChoreCard(chore));
    counts.set(chore.status, (counts.get(chore.status) ?? 0) + 1);
  }

  for (let [key, value] of counts) {
    let countSpan = document.querySelector("#" + key + "-chores");
    if (countSpan == null) {
      return;
    }

    countSpan.textContent = value + " " + key;
  }
}

const updateChores = async (): Promise<void> => {
  await setChores();

  setTimeout(updateChores, 10000);
}

type Flash = {
  id: number;
  contents: string;
  created_at: number;
};

type GetFlashesResponse = {
  success: boolean;
  error?: string;
  flashes: Flash[];
};

const createFlash = (flash: Flash): Node => {
  let callout = document.createElement("div");
  callout.classList.add("callout");
  callout.classList.add("primary");
  callout.setAttribute("data-closeable", "");

  let content = document.createElement("h5");
  content.textContent = flash.contents;
  callout.appendChild(content);

  let createTime = document.createElement("p");
  createTime.textContent = "Created at " + (new Date(flash.created_at * 1000)).toLocaleString();
  callout.appendChild(createTime);

  let button = document.createElement("button");
  button.classList.add("close-button");
  button.setAttribute("aria-label", "Dismiss alert");
  button.type = "button";
  button.setAttribute("data-close", "");

  button.onclick = async (): Promise<void> => {
    const data = new URLSearchParams();
    data.append("id", flash.id.toString());

    await fetch("/api/flashes/dismiss", {
      method: "POST",
      body: data,
    });

    await setFlashes();
  };

  let x = document.createElement("span");
  x.setAttribute("aria-hidden", "true");
  x.innerHTML = "&times;";

  button.appendChild(x);

  callout.appendChild(button);

  return callout;
}

const setFlashes = async (): Promise<void> => {
  let response = await fetch("/api/flashes");
  let flashes = (await response.json()).flashes;

  let flashesNode = document.querySelector("#flashes");
  if (flashesNode == null) {
    return;
  }

  removeAllChildren(flashesNode);

  for (let flash of flashes) {
    flashesNode.appendChild(createFlash(flash));
  }
}

const updateFlashes = async (): Promise<void> => {
  await setFlashes();

  setTimeout(updateFlashes, 10000);
}

const sendFlash = async (): Promise<void> => {
  let messageNode = document.querySelector("#flash-contents");
  if (messageNode == null) {
    return;
  }

  const contents = (<HTMLInputElement>messageNode).value;

  if (contents === "") {
    return;
  }

  (<HTMLInputElement>messageNode).value = "";

  let data = new URLSearchParams();
  data.append("contents", contents);

  let response = await fetch("/api/flashes", {
    method: "POST",
    body: data,
  });

  await setFlashes();

  (<any>$("#add-flash-modal")).foundation("close");
}

const possiblySendFlash = async (event: KeyboardEvent): Promise<void> => {
  if (event.key === "Enter") {
    await sendFlash();
  }
}

(<any>$(document)).foundation();

updateChores();
updateFlashes();
