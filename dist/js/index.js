"use strict";
const createCard = (cardType, title, titleColor, contents) => {
    let cell = document.createElement("div");
    cell.classList.add("cell");
    cell.classList.add("large-auto");
    cell.classList.add("card-type-" + cardType);
    let card = document.createElement("div");
    card.classList.add("card");
    let cardDivider = document.createElement("div");
    cardDivider.classList.add("card-divider");
    cardDivider.classList.add("callout");
    cardDivider.classList.add(titleColor);
    if (title instanceof Node) {
        cardDivider.appendChild(title);
    }
    else {
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
};
const fetchChores = async () => {
    let response = await fetch("/api/chores");
    return (await response.json()).chores;
};
const createChoreCard = (chore) => {
    let title = document.createElement("h2");
    if (chore.status === "completed" || chore.status === "missed") {
        let struckOut = document.createElement("s");
        struckOut.textContent = chore.title;
        title.appendChild(struckOut);
    }
    else {
        title.textContent = chore.title;
    }
    let titleColor = "secondary";
    if (chore.status === "completed") {
        titleColor = "success";
    }
    else if (chore.status === "upcoming") {
        titleColor = "secondary";
    }
    else if (chore.status === "missed") {
        titleColor = "alert";
    }
    else if (chore.status === "overdue") {
        titleColor = "warning";
    }
    else {
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
        contents.push(completeButton);
    }
    return createCard("chore", title, titleColor, contents);
};
const removeCardsOfType = (parent, cardType) => {
    const className = "card-type-" + cardType;
    parent.replaceChildren(...[...parent.children].filter(el => !el.classList.contains("card-type-" + cardType)));
};
const CHORE_FINAL_STATES = ["completed", "missed"];
const setChores = async () => {
    var _a;
    let chores = await fetchChores();
    let cardsNode = document.querySelector("#cards");
    if (cardsNode == null) {
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
    removeCardsOfType(cardsNode, "chore");
    let counts = new Map([
        ["assigned", 0],
        ["upcoming", 0],
        ["overdue", 0],
        ["missed", 0],
        ["completed", 0],
    ]);
    for (let chore of chores) {
        cardsNode.appendChild(createChoreCard(chore));
        counts.set(chore.status, ((_a = counts.get(chore.status)) !== null && _a !== void 0 ? _a : 0) + 1);
    }
    sortCards(cardsNode);
    for (let [key, value] of counts) {
        let countSpan = document.querySelector("#" + key + "-chores");
        if (countSpan == null) {
            return;
        }
        countSpan.textContent = value + " " + key;
    }
};
const updateChores = async () => {
    await setChores();
    setTimeout(updateChores, 10000);
};
const createFlash = (flash) => {
    let contents = [];
    let content = document.createElement("h5");
    content.textContent = flash.contents;
    contents.push(content);
    let createTime = document.createElement("p");
    createTime.textContent = "Created at " + (new Date(flash.created_at * 1000)).toLocaleString();
    contents.push(createTime);
    return createCard("flash", "Message", "primary", contents);
};
const setFlashes = async () => {
    let response = await fetch("/api/flashes");
    let flashes = (await response.json()).flashes;
    let cardsNode = document.querySelector("#cards");
    if (cardsNode == null) {
        return;
    }
    removeCardsOfType(cardsNode, "flash");
    for (let flash of flashes) {
        cardsNode.appendChild(createFlash(flash));
    }
    sortCards(cardsNode);
};
const updateFlashes = async () => {
    await setFlashes();
    setTimeout(updateFlashes, 10000);
};
const sendFlash = async () => {
    let messageNode = document.querySelector("#flash-contents");
    if (messageNode == null) {
        return;
    }
    const contents = messageNode.value;
    if (contents === "") {
        return;
    }
    messageNode.value = "";
    let data = new URLSearchParams();
    data.append("contents", contents);
    let response = await fetch("/api/flashes", {
        method: "POST",
        body: data,
    });
    await setFlashes();
    $("#add-flash-modal").foundation("close");
};
const possiblySendFlash = async (event) => {
    if (event.key === "Enter") {
        await sendFlash();
    }
};
const createMetarCard = (station, metar) => {
    let temperatureText = document.createElement("p");
    temperatureText.textContent = "Temperature: " + metar.temperature + "\u00b0C";
    let pressureText = document.createElement("p");
    pressureText.textContent = "Pressure: " + metar.pressure + " hPa";
    let metarText = document.createElement("p");
    metarText.textContent = metar.metar;
    return createCard("metar", station, "primary", [temperatureText, pressureText, metarText]);
};
const setMetars = async () => {
    let response = await fetch("/api/metars");
    let stations = (await response.json()).stations;
    let cardsNode = document.querySelector("#cards");
    if (cardsNode == null) {
        return;
    }
    removeCardsOfType(cardsNode, "metar");
    for (let station in stations) {
        cardsNode.appendChild(createMetarCard(station, stations[station]));
    }
    sortCards(cardsNode);
};
const sortCards = (parent) => {
    const PRIORITIES = new Map([
        ["card-type-flash", 0],
        ["card-type-metar", 1],
        ["card-type-chore", 2],
    ]);
    [...parent.children]
        .sort((aElement, bElement) => {
        const a = aElement;
        const b = bElement;
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
        }
        else if (aPriority < bPriority) {
            return -1;
        }
        else {
            return 0;
        }
    })
        .forEach(child => parent.appendChild(child));
};
const updateMetars = async () => {
    await setMetars();
    setTimeout(updateMetars, 10000);
};
$(document).foundation();
updateChores();
updateFlashes();
updateMetars();
