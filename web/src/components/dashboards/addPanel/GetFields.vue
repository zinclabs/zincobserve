<template>
    <div class="col" style="height: 100%">
        <div class="q-pa-sm">
            <span class="text-weight-bold">Fields</span>
        </div>
        <div
            class="scroll"
            style="width: 100%;"
            v-if="true"
        >
            <index-list/>
        </div>
    </div>
</template>

<script lang="ts">
import { defineComponent, onMounted, ref } from "vue"
import { useQuasar, date } from "quasar";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import IndexList from "./IndexList.vue"
import useDashboardPanelData from "../../../composables/useDashboardPanel";


    export default defineComponent({
        name: "GetFields",
        props:[],
        components: {
            IndexList,
        },
        setup(){
            const store = useStore();
            const router = useRouter();
            const $q = useQuasar();
            const { t } = useI18n();
            const {dashboardPanelData} = useDashboardPanelData()

            return {
                store,
                router,
                dashboardPanelData              
            }
        }
          
        
    })
</script>

<style lang="sass" scoped>

</style>