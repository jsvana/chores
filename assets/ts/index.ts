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

const createCard = (cardType: string, title: string | Node, titleColor: string, contents: Node[]): Node => {
  let cell = document.createElement("div");
  cell.classList.add("cell");
  cell.classList.add("card-type-" + cardType);

  let card = document.createElement("div");
  card.classList.add("card");

  let cardDivider = document.createElement("div");
  cardDivider.classList.add("card-divider");
  cardDivider.classList.add(titleColor);

  if (title instanceof Node) {
    cardDivider.appendChild(title);
  } else {
    let titleNode = document.createElement("h3");
    titleNode.textContent = title;
    cardDivider.appendChild(titleNode);
  }

  card.appendChild(cardDivider);

  let cardContent = document.createElement("div");
  cardContent.classList.add("card-section");

  for (let content of contents) {
    cardContent.appendChild(content);
  }

  card.appendChild(cardContent);

  cell.appendChild(card);

  return cell;
}

const fetchChores = async (): Promise<Chore[]> => {
  let response = await fetch("/api/chores");
  return (await response.json()).chores;
}

const createChoreCard = (chore: Chore): Node => {
  let title = document.createElement("h3");
  if (chore.status === "completed" || chore.status === "missed") {
    let struckOut = document.createElement("s");
    struckOut.textContent = chore.title;
    title.appendChild(struckOut);
  } else {
    title.textContent = chore.title;
  }

  let titleColor = "secondary";
  if (chore.status === "completed") {
    titleColor = "success";
  } else if (chore.status === "upcoming") {
    titleColor = "secondary";
  } else if (chore.status === "missed") {
    titleColor = "alert";
  } else if (chore.status === "overdue") {
    titleColor = "warning";
  } else {
    titleColor = "primary";
  }

  let contents = [];

  let choreStatusText = "Status: " + chore.status;
  let choreStatus = document.createElement("p");
  choreStatus.textContent = choreStatusText;
  contents.push(choreStatus);

  let expectedDate = new Date(chore.expected_completion_time * 1000);
  let expectedTime = document.createElement("p");
  let expectedTimeBold = document.createElement("strong");
  expectedTimeBold.textContent = "Due date: " + expectedDate.toLocaleString();
  expectedTime.appendChild(expectedTimeBold);
  contents.push(expectedTime);

  let description = document.createElement("p");
  description.textContent = chore.description;
  contents.push(description);

  if (chore.status === "assigned" || chore.status === "overdue") {
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

    contents.push(completeButton);
  }

  return createCard("chore", title, titleColor, contents);
}

const removeCardsOfType = (parent: HTMLElement, cardType: string): void => {
  const className = "card-type-" + cardType;
  parent.replaceChildren(...[...parent.children].filter(el => !el.classList.contains("card-type-" + cardType)));
}

const CHORE_FINAL_STATES = ["completed", "missed"];

const setChores = async (): Promise<void> => {
  let chores = await fetchChores();

  let cardsNode = document.querySelector("#cards");
  if (cardsNode == null) {
    return;
  }

  chores.sort((a: Chore, b: Chore): number => {
    if (a.status == b.status) {
      return a.expected_completion_time - b.expected_completion_time;
    }

    const PRIORITIES: Map<string, number> = new Map([
      ["overdue", 0],
      ["assigned", 1],
      ["upcoming", 2],
      ["missed", 3],
      ["completed", 4],
    ]);

    const aPriority = PRIORITIES.get(a.status);
    const bPriority = PRIORITIES.get(b.status);
    if (aPriority == null) {
      return -1;
    }
    if (bPriority == null) {
      return 1;
    }

    if (aPriority > bPriority) {
      return 1;
    } else if (aPriority < bPriority) {
      return -1;
    } else {
      return 0;
    }
  });

  removeCardsOfType(<HTMLElement>cardsNode, "chore");

  for (let chore of chores) {
    cardsNode.appendChild(createChoreCard(chore));
  }

  sortCards(<HTMLElement>cardsNode);
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
  let contents = [];
  let content = document.createElement("h5");
  content.textContent = flash.contents;
  contents.push(content);

  let createTime = document.createElement("p");
  createTime.style.fontSize = "0.8em";
  createTime.textContent = "Created at " + (new Date(flash.created_at * 1000)).toLocaleString();
  contents.push(createTime);

  let dismiss = document.createElement("button");
  dismiss.type = "button";
  dismiss.classList.add("button");
  dismiss.classList.add("primary");
  dismiss.classList.add("expanded");
  dismiss.classList.add("large");
  dismiss.textContent = "Dismiss";
  dismiss.onclick = async (): Promise<void> => {
      const data = new URLSearchParams();
      data.append("id", flash.id.toString());

      await fetch("/api/flashes/dismiss", {
        method: "POST",
        body: data,
      });

      await setFlashes();
  };

  contents.push(dismiss);

  return createCard("flash", "Message", "success", contents);
}

const setFlashes = async (): Promise<void> => {
  let response = await fetch("/api/flashes");
  let flashes = (await response.json()).flashes;

  let cardsNode = document.querySelector("#cards");
  if (cardsNode == null) {
    return;
  }

  removeCardsOfType(<HTMLElement>cardsNode, "flash");

  for (let flash of flashes) {
    cardsNode.appendChild(createFlash(flash));
  }

  sortCards(<HTMLElement>cardsNode);
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

type Weather = {
  conditions: string[];
  intensity: string;
};

type StationMetar = {
  metar: string;
  pressure?: number;
  temperature?: number;
  weather: Weather[];
};

type GetMetarsResponse = {
  stations: Map<string, StationMetar>;
};

const createMetarCard = (station: string, metar: StationMetar): Node => {
  let temperatureText = document.createElement("p");
  temperatureText.textContent = "Temperature: " + metar.temperature + "\u00b0C";

  let pressureText = document.createElement("p");
  pressureText.textContent = "Pressure: " + metar.pressure + " hPa";

  let metarText = document.createElement("p");
  metarText.textContent = metar.metar;

  return createCard("metar", station, "primary", [temperatureText, pressureText, metarText]);
}

const setMetars = async (): Promise<void> => {
  let response = await fetch("/api/metars");
  let stations = (await response.json()).stations;

  let cardsNode = document.querySelector("#cards");
  if (cardsNode == null) {
    return;
  }

  let stationNames = Object.keys(stations).sort();

  removeCardsOfType(<HTMLElement>cardsNode, "metar");

  for (let stationName of stationNames) {
    cardsNode.appendChild(createMetarCard(stationName, stations[stationName]));
  }

  sortCards(<HTMLElement>cardsNode);
}

const sortCards = (parent: HTMLElement): void => {
  const PRIORITIES: Map<string, number> = new Map([
    ["card-type-flash", 0],
    ["card-type-metar", 1],
    ["card-type-chore", 2],
  ]);

  [...parent.children]
  .sort((aElement: Element, bElement: Element): number => {
    const a = <HTMLElement>aElement;
    const b = <HTMLElement>bElement;
    const aType = [...a.classList.values()].find(c => c.startsWith('card-type-'));
    const bType = [...b.classList.values()].find(c => c.startsWith('card-type-'));
    if (aType == null) {
      return -1;
    }
    if (bType == null) {
      return 1;
    }

    const aPriority = PRIORITIES.get(aType);
    const bPriority = PRIORITIES.get(bType);
    if (aPriority == null) {
      return -1;
    }
    if (bPriority == null) {
      return 1;
    }

    if (aPriority > bPriority) {
      return 1;
    } else if (aPriority < bPriority) {
      return -1;
    } else {
      return 0;
    }
  })
  .forEach(child=>parent.appendChild(child));
}

const updateMetars = async (): Promise<void> => {
  await setMetars();

  setTimeout(updateMetars, 10000);
}

(<any>$(document)).foundation();

updateChores();
updateFlashes();
updateMetars();
