type Chore = {
  title: string;
  description: string;
  expected_completion_time: number;
  overdue: boolean;
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
  let title = document.createElement("h2");
  if (chore.status === "completed" || chore.status === "missed") {
    let struckOut = document.createElement("s");
    struckOut.textContent = chore.title;
    title.appendChild(struckOut);

    if (chore.status === "missed") {
      let missed = document.createElement("span");
      missed.classList.add("label");
      missed.classList.add("alert");
      missed.textContent = "MISSED";
      missed.style.fontSize = "1em";

      title.appendChild(missed);
    }
  } else {
    title.textContent = chore.title;

    if (chore.overdue) {
      let overdue = document.createElement("span");
      overdue.classList.add("label");
      overdue.classList.add("warning");
      overdue.textContent = "OVERDUE";
      overdue.style.fontSize = "1em";

      title.appendChild(overdue);
    }
  }

  cardDivider.appendChild(title);
  card.appendChild(cardDivider);

  let cardContent = document.createElement("div");

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
    completeButton.textContent = "Mark Completed";

    completeButton.onclick = async (): Promise<void> => {
      const data = new URLSearchParams();
      data.append("title", chore.title);
      data.append("expected_completion_time", chore.expected_completion_time.toString());

      await fetch("/api/chores/complete", {
        method: "POST",
        body: data,
      });
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
  if (choresNode === null) {
    return;
  }

  chores.sort((a: Chore, b: Chore): number => {
    if (CHORE_FINAL_STATES.includes(a.status) && b.status === "assigned") {
      return 1;
    }

    if (CHORE_FINAL_STATES.includes(b.status) && a.status === "assigned") {
      return -1;
    }

    return b.expected_completion_time - a.expected_completion_time;
  });

  removeAllChildren(choresNode);

  for (let chore of chores) {
    choresNode.appendChild(createChoreCard(chore));
  }
}

const updateChores = async (): Promise<void> => {
  await setChores();

  setTimeout(updateChores, 10000);
}

updateChores();
