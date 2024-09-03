<template>
  <div :class="className" :style="{height:height,width:width}" />
</template>

<script>
import echarts from 'echarts'
import * as macarons from 'echarts/theme/macarons'
import resize from './mixins/resize'
import * as datetime from '@/utils/datetime.js'

export default {
  mixins: [resize],
  props: {
    className: {
      type: String,
      default: 'chart'
    },
    width: {
      type: String,
      default: '100%'
    },
    height: {
      type: String,
      default: '250px'
    },
    autoResize: {
      type: Boolean,
      default: true
    },
    chartData: {
      type: Object,
      required: true
    }
  },
  data() {
    return {
      chart: null
    }
  },
  watch: {
    chartData: {
      deep: true,
      handler(val) {
        this.setOptions(val)
      }
    }
  },
  mounted() {
    this.$nextTick(() => {
      this.initChart()
    })
  },
  beforeDestroy() {
    if (!this.chart) {
      return
    }
    this.chart.dispose()
    this.chart = null
  },
  methods: {
    initChart() {
      this.chart = echarts.init(this.$el, 'macarons')
      this.setOptions(this.chartData)
    },
    setOptions({ data, title } = {}) {
      console.log(data)
      const xaxis = data.map(t => new Date(t.timestamp).strftime('%H:%m:%S'))
      const kerndata = data.map(t => t.kernel_cpu_usages / t.now_cpu_time * 100.0)
      const userdata = data.map(t => t.user_cpu_usages / t.now_cpu_time * 100.0)
      this.chart.setOption({
        title: {
          text: this.$t('CPU使用率'),
          x: 'left'
        },
        xAxis: {
          data: xaxis,
          boundaryGap: false,
          axisTick: {
            show: false
          }
        },
        grid: {
          left: 10,
          right: 10,
          bottom: 20,
          top: 40,
          containLabel: true
        },
        tooltip: {
          trigger: 'axis',
          axisPointer: {
            type: 'cross'
          },
          formatter: function(it) {
            return it.map(t => t.marker + t.seriesName + ': ' + t.value.toFixed(2) + '%').join('<br/>')
          },
          padding: [5, 10]
        },
        yAxis: {
          axisTick: {
            show: false
          },
          max: function(value) {
            var max_t = Math.round(value.max)
            if (max_t < 10) {
              return 10
            } else {
              return max_t
            }
          }
        },
        legend: {
          data: [this.$t('系统CPU使用率'), this.$t('用户CPU使用率')],
          left: 'right',
          top: '5px'
        },
        series: [{
          name: this.$t('系统CPU使用率'), itemStyle: {
            normal: {
              color: '#FF005A',
              lineStyle: {
                color: '#FF005A',
                width: 2
              }
            }
          },
          smooth: true,
          type: 'line',
          data: kerndata,
          animationDuration: 2800,
          animationEasing: 'cubicInOut'
        }, {
          name: this.$t('用户CPU使用率'), itemStyle: {
            normal: {
              color: '#4455CA',
              lineStyle: {
                color: '#4455CA',
                width: 2
              }
            }
          },
          smooth: true,
          type: 'line',
          data: userdata,
          animationDuration: 2800,
          animationEasing: 'cubicInOut'
        }]
      })
    }
  }
}
</script>
