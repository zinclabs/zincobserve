<!-- eslint-disable vue/no-unused-components -->
<template>
  <div style="height: calc(100vh - 57px); overflow-y: auto" class="scroll">
    <div class="flex justify-between items-center q-pa-sm">
      <div class="q-table__title q-mr-md">
        {{ editMode ? t("panel.editPanel") : t("panel.addPanel") }}
      </div>
      <div class="flex items-baseline q-gutter-sm">
        <date-time @date-change="updateDateTime" />
        <q-btn
          class="q-ml-md q-mb-xs text-bold"
          outline
          padding="sm lg"
          color="red"
          no-caps
          :label="t(`Discard`)"
          @click="goBackToDashboardList"
        />
        <q-btn
          class="q-ml-md q-mb-xs text-bold"
          outline
          padding="sm lg"
          color="white"
          text-color="black"
          no-caps
          :label="t(`Save`)"
          @click="savePanelOnClick"
        />
        <q-btn
          class="q-ml-md q-mb-xs text-bold no-border"
          padding="sm lg"
          color="secondary"
          no-caps
          :label="t(`Apply`)"
          @click="runQuery"
        />
      </div>
    </div>
    <q-separator></q-separator>
    <div class="row" style="height: calc(100vh - 118px); overflow-y: auto">
      <div
        class="col"
        style="overflow: hidden; min-width: 75px; max-width: 75px"
      >
        <ChartSelection
          v-model:selectedChartType="dashboardPanelData.data.type"
        />
      </div>
      <q-separator vertical />
      <div class="col scroll" style="height: 100%">
        <GetFields />
      </div>
      <q-separator vertical />
      <div class="col scroll" style="height: 100%">
        <Layout />
      </div>
      <q-separator vertical />
      <div class="col-7 scroll" style="height: 100%">
        <SearchBar />
        <q-separator />
        <ChartRender
          :data="chartData"
          :selectedTimeDate="dashboardPanelData.meta.dateTime"
        />
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  computed,
  toRaw,
  onActivated,
  nextTick,
} from "vue";
import ChartSelection from "../../../components/dashboards/addPanel/ChartSelection.vue";
import GetFields from "../../../components/dashboards/addPanel/GetFields.vue";
import { useQuasar, date } from "quasar";

import { useI18n } from "vue-i18n";
import {
  addPanel,
  getConsumableDateTime,
  getPanel,
  updatePanel,
} from "../../../utils/commons";
import { useRoute, useRouter } from "vue-router";
import { useStore } from "vuex";
import Layout from "../../../components/dashboards/addPanel/Layout.vue";
import SearchBar from "../../../components/dashboards/SearchBar.vue";
import useDashboardPanelData from "../../../composables/useDashboardPanel";
import DateTime from "../../../plugins/logs/DateTime.vue";
import ChartRender from "../../../components/dashboards/addPanel/ChartRender.vue";

export default defineComponent({
  name: "AddPanel",
  components: {
    ChartSelection,
    GetFields,
    Layout,
    SearchBar,
    DateTime,
    ChartRender,
  },
  setup() {
    // This will be used to copy the chart data to the chart renderer component
    // This will deep copy the data object without reactivity and pass it on to the chart renderer
    const chartData = ref({});
    const $q = useQuasar();
    const { t } = useI18n();
    const router = useRouter();
    const route = useRoute();
    const store = useStore();
    const { dashboardPanelData, resetDashboardPanelData } =
      useDashboardPanelData();
    const editMode = ref(false);

    onActivated(async () => {
      // todo check for the edit more
      if (route.query.panelId) {
        editMode.value = true;
        const panelData = await getPanel(
          store,
          route.query.dashboard,
          route.query.panelId
        );
        console.log("panel data", panelData);
        Object.assign(dashboardPanelData.data, panelData);
        runQuery();
      } else {
        editMode.value = false;
        resetDashboardPanelData();
        chartData.value = {};
      }
    });

    let list = computed(function () {
      return [toRaw(store.state.currentSelectedDashboard)];
    });

    const currentDashboard = toRaw(store.state.currentSelectedDashboard);

    const getDashboard = () => {
      return currentDashboard.dashboardId;
    };

    const runQuery = () => {
      console.log("query change detected to run");

      // copy the data object excluding the reactivity
      chartData.value = JSON.parse(JSON.stringify(dashboardPanelData.data));
    };

    const updateDateTime = (value: object) => {
      dashboardPanelData.meta.dateTime = getConsumableDateTime(value);
    };
    const goBack = () => {
      return router.push({
        path: "/viewDashboard",
        query: { dashboard: route.query.dashboard },
      });
    };

    const isValid = () => {
      const error = []
      const dashboardData = dashboardPanelData.data

      if(!dashboardData.fields.x.length){
        error.push("Please add at least one field in X axis")
      }

      if(!dashboardData.fields.y.length){
        error.push("Please add at least one field in Y axis")
      }

      if(dashboardData.type == "pie" && dashboardData.fields.y.length > 1 ){
        error.push("You can add only one field in the Y axis while, selected chart type is pie")
      }

      if(dashboardData.fields.y.length && dashboardData.fields.y.filter((it:any) => (it.aggregationFunction == null || it.aggregationFunction == '')).length){
        error.push("Aggregation function required")
      }

      if(dashboardData.fields.y.length && dashboardData.fields.y.filter((it:any) => (it.label == null || it.label == '')).length){
        error.push("label required")
      }

      if(dashboardData.config.title == null || dashboardData.config.title == '' ){
        error.push("Panel Name is required")
      }

      if(!dashboardData.fields.filter.length){

        if(dashboardData.fields.filter.filter((it:any) => ((it.type == "list" && !it.values?.length))).length){
          error.push("Please select at least one list filter value")
        }

        if(dashboardData.fields.filter.filter((it:any) => (it.type == "condition" && it.operator == null)).length){
          error.push("Please select at least one condition operator value")
        }

        if(dashboardData.fields.filter.filter((it:any) => (it.type == "condition" && (it.value == null || it.value == ''))).length){
          error.push("Please select at least one condition value")
        }

      }
      for (let index = 0; index < error.length; index++) {
        $q.notify({
          type: "negative",
          message: error[index],
          timeout: 5000,
        });
      }
     
      if(error.length){
          return false
        }else{
          return true
        }

    }

    const savePanelChangesToDashboard = async (dashId: string) => {
      if(!isValid()){
        return
      }
      if (editMode.value) {
        await updatePanel(
          store,
          dashId,
          JSON.parse(JSON.stringify(dashboardPanelData.data))
        );
      } else {
        const panelId =
          "Panel_ID" + Math.floor(Math.random() * (99999 - 10 + 1)) + 10;

        dashboardPanelData.data.id = panelId;
        await addPanel(
          store,
          dashId,
          JSON.parse(JSON.stringify(dashboardPanelData.data))
        );
      }

      await nextTick();
      return router.push({
        path: "/viewDashboard",
        query: { dashboard: dashId },
      });
    };

    return {
      t,
      updateDateTime,
      goBack,
      savePanelChangesToDashboard,
      runQuery,
      getDashboard,
      currentDashboard,
      list,
      dashboardPanelData,
      chartData,
      editMode,
    };
  },
  methods: {
    goBackToDashboardList(evt: any, row: any) {
      this.goBack();
    },
    savePanelOnClick() {
      const dashboardId = this.$route.query.dashboard + ""
      this.savePanelChangesToDashboard(dashboardId);
    },
  },
});
</script>

<style lang="sass" scoped>
</style>