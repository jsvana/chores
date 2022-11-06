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
    }
    else {
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
    let description = document.createElement("p");
    description.textContent = chore.description;
    cardContent.appendChild(description);
    let expectedDate = new Date(chore.expected_completion_time * 1000);
    let expectedTime = document.createElement("p");
    expectedTime.textContent = expectedDate.toLocaleString();
    cardContent.appendChild(expectedTime);
    if (chore.status === "assigned") {
        let completeButton = document.createElement("button");
        completeButton.type = "button";
        completeButton.classList.add("button");
        completeButton.classList.add("success");
        completeButton.textContent = "Mark Completed";
        completeButton.onclick = async () => {
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
};
const removeAllChildren = (parent) => {
    while (parent.firstChild) {
        parent.removeChild(parent.firstChild);
    }
};
const setChores = async () => {
    let chores = await fetchChores();
    let choresNode = document.querySelector("#chores");
    if (choresNode === null) {
        return;
    }
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
