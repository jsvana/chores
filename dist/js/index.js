"use strict";
const fetchChores = async () => {
    let response = await fetch("/api/chores");
    return (await response.json()).chores;
};
const createChoreCard = (chore) => {
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
    }
    else if (chore.status === "missed") {
        cardDivider.classList.add("alert");
    }
    else if (chore.overdue) {
        cardDivider.classList.add("warning");
    }
    else {
        cardDivider.classList.add("primary");
    }
    let title = document.createElement("h2");
    if (chore.status === "completed" || chore.status === "missed") {
        let struckOut = document.createElement("s");
        struckOut.textContent = chore.title;
        title.appendChild(struckOut);
    }
    else {
        title.textContent = chore.title;
    }
    cardDivider.appendChild(title);
    card.appendChild(cardDivider);
    let cardContent = document.createElement("div");
    cardContent.classList.add("card-section");
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
        completeButton.onclick = async () => {
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
};
const removeAllChildren = (parent) => {
    while (parent.firstChild) {
        parent.removeChild(parent.firstChild);
    }
};
const CHORE_FINAL_STATES = ["completed", "missed"];
const setChores = async () => {
    let chores = await fetchChores();
    let choresNode = document.querySelector("#chores");
    if (choresNode === null) {
        return;
    }
    chores.sort((a, b) => {
        if (CHORE_FINAL_STATES.includes(a.status) && b.status === "assigned") {
            return 1;
        }
        if (CHORE_FINAL_STATES.includes(b.status) && a.status === "assigned") {
            return -1;
        }
        return a.expected_completion_time - b.expected_completion_time;
    });
    removeAllChildren(choresNode);
    for (let chore of chores) {
        choresNode.appendChild(createChoreCard(chore));
    }
};
const updateChores = async () => {
    await setChores();
    setTimeout(updateChores, 10000);
};
updateChores();
