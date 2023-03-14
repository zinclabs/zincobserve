// Copyright 2022 Zinc Labs Inc. and Contributors

//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at

//      http:www.apache.org/licenses/LICENSE-2.0

//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License. 

//Dashboard Manipulation Functions

import dashboardService from "../services/dashboards";
import { toRaw } from "vue";
import { date } from "quasar";
import moment from "moment";

export const deletePanelFromDashboard = async (
  store: any,
  dashboardId: String,
  panelId: any,
  dashboardList: any
) => {
  let dashboardPos = 0;
  let counter = 0;
  let found = 0;
  let newDashboardList = JSON.parse(JSON.stringify(toRaw(dashboardList)));
  let currentDashboard = JSON.parse(
    JSON.stringify(toRaw(store.state.currentSelectedDashboard))
  );
  let currentPanelsData = JSON.parse(
    JSON.stringify(toRaw(store.state.currentPanelsData))
  );

  for (const dashboard of dashboardList) {
    if (dashboardId === dashboard.name) {
      let modDashboardObject = JSON.parse(toRaw(dashboard.details));
      for (const panel of modDashboardObject.panels) {
        if (panelId === panel.id) {
          // modDashboardObject.panels[counter] = panelObject
          modDashboardObject.panels.splice(counter, 1);
          currentDashboard.panels = modDashboardObject.panels;

          //remove layout from current dashboard
          const layoutIndex = modDashboardObject.layouts.findIndex(
            (layout: any) => layout.panelId == panelId
          );
          modDashboardObject.layouts.splice(layoutIndex, 1);
          currentDashboard.layouts = modDashboardObject.layouts;

          //remove panelData from current panel
          const panelIndex = currentPanelsData.findIndex(
            (panel: any) => panel.id == panelId
          );
          currentPanelsData.splice(panelIndex, 1);

          found++;
        }
        counter++;
      }
      if (found === 0) {
        console.log("Not Deleted Because not found");
        //modDashboardObject.panels.push(toRaw(panelObject))
      }
      //dashboardList[counter].details = JSON.stringify(modDashboardObject)
      newDashboardList[dashboardPos] = {
        name: dashboardId,
        details: JSON.stringify(toRaw(modDashboardObject)),
      };
    }
    dashboardPos++;
  }

  store.dispatch("setCurrentSelectedDashboard", currentDashboard);
  store.dispatch("setAllCurrentDashboards", newDashboardList);
  store.dispatch("setCurrentPanelsData", currentPanelsData);

  // getCurrentDashboard(store,dashboardId)
};

export const updateAllDashboardsFromBackend = async (store: any) => {
  await dashboardService
    .list(0, 1000, "name", false, "",store.state.selectedOrganization.identifier)
    .then((res) => {
      store.dispatch("setAllCurrentDashboards", res.data.list);
    });
};

export const getCurrentDashboard = async (store: any, dashboardId: String) => {
  await updateAllDashboardsFromBackend(store);
  const dashboardList = toRaw(store.state.allCurrentDashboards);
  for (const dashboard of dashboardList) {
    if (dashboardId === dashboard.name) {
      store.dispatch(
        "setCurrentSelectedDashboard",
        JSON.parse(dashboard.details)
      );
      return JSON.parse(dashboard.details);
    }
  }
};

export const getCurrentPanel = async (
  store: any,
  dashboardId: String,
  panelId: String
) => {
  //await updateAllDashboardsFromBackend(store)
  const dashboardList = toRaw(store.state.allCurrentDashboards);
  for (const dashboard of dashboardList) {
    if (dashboardId === dashboard.name) {
      for (const panel of JSON.parse(toRaw(dashboard.details)).panels) {
        if (panelId === panel.id) {
          return toRaw(panel);
        }
      }
    }
  }
};

export const setCurrentDashboard = (dashboardId: String, panelId: String) => {
  return "AAAA";
};

export const setCurrentPanelToDashboardList = (
  dashboardList: any,
  dashboardId: String,
  panelId: String,
  panelObject: any,
  store: any,
  newLayoutObj: any
) => {
  let dashboardPos = 0;
  let counter = 0;
  let found = 0;
  for (const dashboard of dashboardList) {
    if (dashboardId === dashboard.name) {
      console.log("Inside " + counter);
      let modDashboardObject = JSON.parse(toRaw(dashboard.details));
      console.log("modDashboardObject---",modDashboardObject);
      
      if (modDashboardObject.layouts && modDashboardObject.layouts.length > 0) {
        modDashboardObject.layouts.push(newLayoutObj);
      } else {
        modDashboardObject.layouts = [newLayoutObj];
      }
      for (const panel of modDashboardObject.panels) {
        if (panelId === panel.id) {
          console.log("Inside Panel match  -- " + panelId);
          console.log("Panel Replace  --- " + JSON.stringify(panel));
          modDashboardObject.panels[counter] = panelObject;
          found++;
        }
        counter++;
      }
      if (found === 0) {
        modDashboardObject.panels.push(toRaw(panelObject));
      }
      //dashboardList[counter].details = JSON.stringify(modDashboardObject)
      dashboardList[dashboardPos] = {
        name: dashboardId,
        details: JSON.stringify(toRaw(modDashboardObject)),
      };
    }
    dashboardPos++;
  }
  console.log("query dashboardList ====",dashboardList);
  store.dispatch("setAllCurrentDashboards", dashboardList);
};

export const getDateConsumableDateTime = function (dateVal: {
  tab: string;
  selectedRelativePeriod: string;
  selectedRelativeValue: number;
  startDate: string;
  startTime: string;
  endDate: string;
  endTime: string;
}) {
  console.log("==dateVal===",dateVal);
  
  if (dateVal.tab == "relative") {
    console.log('r');
    
    let period = "";
    let periodValue = 0;
    // quasar does not support arithmetic on weeks. convert to days.
    if (dateVal.selectedRelativePeriod.toLowerCase() == "weeks") {
      period = "days";
      periodValue = dateVal.selectedRelativeValue * 7;
    } else {
      period = dateVal.selectedRelativePeriod.toLowerCase();
      periodValue = dateVal.selectedRelativeValue;
    }
    const subtractObject = '{"' + period + '":' + periodValue + "}";

    const endTimeStamp = new Date();
    const startTimeStamp = date.subtractFromDate(
      endTimeStamp,
      JSON.parse(subtractObject)
    );

    return {
      start_time: startTimeStamp,
      end_time: endTimeStamp,
    };
  } else {
    console.log('a');
    let start, end;

    if (dateVal.startDate == "" && dateVal.startTime == "") {
      start = new Date();
    } else {
      start = new Date(dateVal.startDate + " " + dateVal.startTime);
    }

    if (dateVal.endDate == "" && dateVal.endTime == "") {
      end = new Date();
    } else {
      end = new Date(dateVal.endDate + " " + dateVal.endTime);
    }

    const rVal = {
      start_time: start,
      end_time: end,
    };
    console.log(rVal);

    return rVal;
  }
};

export function getConsumableDateTime(dateObj: any) {
  if (dateObj.tab == "relative") {
    let period = "";
    let periodValue = 0;
    // quasar does not support arithmetic on weeks. convert to days.

    if (
      dateObj.relative.period.label.toLowerCase() ==
      "weeks"
    ) {
      period = "days";
      periodValue = dateObj.relative.value * 7;
    } else {
      period =
        dateObj.relative.period.label.toLowerCase();
      periodValue = dateObj.relative.value;
    }
    const subtractObject = '{"' + period + '":' + periodValue + "}";

    let endTimeStamp = new Date();
    // if (searchObj.data.resultGrid.currentPage > 0) {
    //   endTimeStamp = searchObj.data.resultGrid.currentDateTime;
    // } else {
    //   searchObj.data.resultGrid.currentDateTime = endTimeStamp;
    // }

    const startTimeStamp = date.subtractFromDate(
      endTimeStamp,
      JSON.parse(subtractObject)
    );

    return {
      start_time: startTimeStamp,
      end_time: endTimeStamp,
    };
  } else {
    let start, end;
    if (
      dateObj.absolute.date.from == "" &&
      dateObj.absolute.startTime == ""
    ) {
      start = new Date();
    } else {
      start = new Date(
        dateObj.absolute.date.from +
          " " +
          dateObj.absolute.startTime
      );
    }
    if (
      dateObj.absolute.date.to == "" &&
      dateObj.absolute.endTime == ""
    ) {
      end = new Date();
    } else {
      end = new Date(
        dateObj.absolute.date.to +
          " " +
          dateObj.absolute.endTime
      );
    }
    const rVal = {
      start_time: start,
      end_time: end,
    };
    return rVal;
  }
}

export const modifySQLQuery = (currentTimeObj: any, querySQL: String) => {
  const startTime = moment(String(currentTimeObj.start_time)).format(
    "YYYY-MM-DDThh:mm:ssZ"
  );
  const endTime = moment(String(currentTimeObj.end_time)).format(
    "YYYY-MM-DDThh:mm:ssZ"
  );
  const replaceString =
    "time_range(_timestamp,'" + startTime + "', '" + endTime + "') ";
  let modString: String = "";

  if (querySQL.match(/time_range[^)]*\)/)) {
    modString = querySQL.replace(/time_range[^)]*\)/, replaceString);
  } else if (querySQL.match(/WHERE/)) {
    modString = querySQL.replace(/WHERE/, "WHERE " + replaceString + " and  ");
  } else {
    modString = querySQL;
  }
  return modString;
};


// ----------- NEW Methods to retrieve dashboard data
export const getAllDashboards = async (store: any) => {
  // api call
  return await dashboardService
  .list(0, 1000, "name", false, "",store.state.selectedOrganization.identifier)
  .then((res) => {
    console.log('dashboard list retriveid');
    console.log(res);
    // save to store
    store.dispatch("setAllDashboardList", res.data.list);
  })
  .catch((error) => {
    console.log(error);
  });
}

export const addPanel = async (store: any, dashboardId: any, panelData: any) => {
// get the object of panel data
// find the dashboard and add the panel data to dashboard object
// call the update dashboard function
  
  if(!store.state.allDashboardList || store.state.allDashboardList.length == 0) {
    await getAllDashboards(store)
  }
  const dashboardList = store.state.allDashboardList
  let currentDashboard = dashboardList.find((it:any)=>it.name === dashboardId )
  currentDashboard = JSON.parse(currentDashboard.details)
  if(!currentDashboard.panels){
    currentDashboard.panels = []
  }
  currentDashboard.panels.push(panelData)

  const maxI =
    currentDashboard.layouts?.length > 0
      ? Math.max(...currentDashboard.layouts?.map((obj:any) => obj.i))
      : 0;
  const maxY =
    currentDashboard.layouts?.length > 0
      ? Math.max(...currentDashboard.layouts?.map((obj:any) => obj.y))
      : 0;

  const newLayoutObj = {
    x: 0,
    y: currentDashboard.layouts?.length > 0 ? maxY + 10 : 0,
    w: 12,
    h: 12,
    i: maxI + 1,
    panelId: panelData.id,
    static: false,
  };

  if(!currentDashboard.layouts){
    currentDashboard.layouts = []
  }
  currentDashboard.layouts.push(newLayoutObj)

  console.log(currentDashboard);
  

  return await updateDashboard(store, store.state.selectedOrganization.identifier, dashboardId, currentDashboard )
}

export const deletePanel = async (store:any, dashboardId:any, panelId:any) => {
// get the object of panel id
// find the dashboard and remove the panel data to dashboard object
// call the update dashboard function
  const dashboardList = store.state.allDashboardList
  let currentDashboard = dashboardList.find((it:any)=>it.name === dashboardId )
  currentDashboard = JSON.parse(currentDashboard.details)

    //remove panel from current dashboard
    const panelIndex = currentDashboard.panels.findIndex(
      (panel: any) => panel.id == panelId
    );
    currentDashboard.panels.splice(panelIndex, 1);
    currentDashboard.panels = currentDashboard.panels;

    //remove layout from current dashboard
    const layoutIndex = currentDashboard.layouts.findIndex(
      (layout: any) => layout.panelId == panelId
    );
    currentDashboard.layouts.splice(layoutIndex, 1);
    currentDashboard.layouts = currentDashboard.layouts;

    await updateDashboard(store, store.state.selectedOrganization.identifier, dashboardId, currentDashboard )
}

export const updatePanel = async (store:any, dashboardId:any, panelData:any) => {
  // get the object of panel id
  // find the dashboard and remove the panel data to dashboard object
  // call the update dashboard function
    const dashboardList = store.state.allDashboardList
    let currentDashboard = dashboardList.find((it:any)=>it.name === dashboardId )
    currentDashboard = JSON.parse(currentDashboard.details)
  
      //remove panel from current dashboard
      const panelIndex = currentDashboard.panels.findIndex(
        (panel: any) => panel.id == panelData.id
      );
      currentDashboard.panels[panelIndex] = panelData;
      currentDashboard.panels = currentDashboard.panels;
  
      await updateDashboard(store, store.state.selectedOrganization.identifier, dashboardId, currentDashboard )
  }

export const updateDashboard = async (store: any, org:any, dashboardId:any, currentDashboardData: any) => {
// make an api call to update the dashboard
  return await dashboardService
    .save(
      org,
      dashboardId,
      JSON.stringify(JSON.stringify(currentDashboardData))
    )
    .then(async (res) => {
    // update dashboardList
       await getAllDashboards(store)
    });
}

export const getDashboard = async (store: any, dashboardId: any) => {
  if(!store.state.allDashboardList || store.state.allDashboardList.length == 0) {
    await getAllDashboards(store)
  }
  const dashboardList = store.state.allDashboardList
  let currentDashboard = dashboardList.find((it:any)=>it.name === dashboardId )
  currentDashboard = JSON.parse(currentDashboard.details)
  return currentDashboard
}

export const getPanel = async (store: any, dashboardId: any, panelId: any) => {
  if(!store.state.allDashboardList || store.state.allDashboardList.length == 0) {
    await getAllDashboards(store)
  }
  const dashboardList = store.state.allDashboardList
  let currentDashboard = dashboardList.find((it:any)=>it.name === dashboardId )
  currentDashboard = JSON.parse(currentDashboard.details)
  return currentDashboard.panels?.find((it: any) => it.id == panelId)
}