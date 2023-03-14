<template>
  <div style="height: 100%">
    <div class="q-pa-none" style="max-width: 75px">
      <q-list separator>
        <q-item
          :class="
            selectedChartType === item.id ? 'q-pa-none bg-grey-3' : 'q-pa-none'
          "
          v-for="(item, index) in ChartsArray"
          :key="index"
          clickable
          v-ripple
          @click="$emit('update:selectedChartType', item.id)"
        >
          <q-item-section class="q-pa-sm">
            <q-icon
              size="sm"
              color="primary"
              :name="item.image"
              class="q-mx-auto q-mb-sm"
            />
            <q-item-label class="q-pa-none q-mx-auto" caption>{{
              item.title
            }}</q-item-label>
          </q-item-section>
        </q-item>
      </q-list>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";
import { getImageURL } from "../../../utils/zincutils";

export default defineComponent({
  name: "ChartSelection",
  props: ["selectedChartType"],
  emits: ["update:selectedChartType"],

  setup() {
    // array of charts
    const chartsArray = ref([
      {
        image: "img:" + getImageURL("images/dashboard/charts/area-graph.png"),
        title: "Area",
        id: "area",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/bar-chart.png"),
        title: "Bar",
        id: "bar",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/bar-graph.png"),
        title: "Horizontal",
        id: "h-bar",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/line-chart.png"),
        title: "Line",
        id: "line",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/pie-chart.png"),
        title: "Pie",
        id: "pie",
      },
      {
        image:
          "img:" + getImageURL("images/dashboard/charts/scatter-graph.png"),
        title: "Scatter",
        id: "scatter",
      },
      {
        image: "img:" + getImageURL("images/dashboard/charts/table.png"),
        title: "Table",
        id: "table",
      },
    ]);

    return {
      ChartsArray: chartsArray,
    };
  },
  components: {},
});
</script>

<style lang="sass" scoped>
</style>